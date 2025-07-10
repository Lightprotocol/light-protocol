use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Ident, Item, ItemEnum, ItemFn, ItemMod, ItemStruct, Result, Token,
};

/// Arguments for the compressible macro (kept for backwards compatibility)
pub(crate) struct CompressibleArgs {}

impl Parse for CompressibleArgs {
    fn parse(_input: ParseStream) -> Result<Self> {
        Ok(CompressibleArgs {})
    }
}

/// The old compressible attribute - now deprecated
pub(crate) fn compressible(_args: CompressibleArgs, input: ItemStruct) -> Result<TokenStream> {
    // Just return the struct as-is, no longer generating modules
    Ok(quote! {
        #input
    })
}

/// Parse a comma-separated list of identifiers
struct IdentList {
    idents: Punctuated<Ident, Token![,]>,
}

impl Parse for IdentList {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(IdentList {
            idents: Punctuated::parse_terminated(input)?,
        })
    }
}

/// Generate compress instructions for the specified account types
pub(crate) fn add_compressible_instructions(
    args: TokenStream,
    mut module: ItemMod,
) -> Result<TokenStream> {
    let ident_list = syn::parse2::<IdentList>(args)?;

    // Check if module has content
    if module.content.is_none() {
        return Err(syn::Error::new_spanned(&module, "Module must have a body"));
    }

    // Get the module content
    let content = module.content.as_mut().unwrap();

    // Collect all struct names for the enum
    let struct_names: Vec<_> = ident_list.idents.iter().collect();

    // Generate the CompressedAccountVariant enum
    let enum_variants = struct_names.iter().map(|name| {
        quote! {
            #name(#name)
        }
    });

    let compressed_variant_enum: ItemEnum = syn::parse_quote! {
        /// Unified enum that can hold any account type
        #[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
        pub enum CompressedAccountVariant {
            #(#enum_variants),*
        }
    };

    // Generate Default implementation
    let first_struct = &struct_names[0];
    let default_impl: Item = syn::parse_quote! {
        impl Default for CompressedAccountVariant {
            fn default() -> Self {
                Self::#first_struct(#first_struct::default())
            }
        }
    };

    // Generate DataHasher implementation
    let hash_match_arms = struct_names.iter().map(|name| {
        quote! {
            Self::#name(data) => data.hash::<H>()
        }
    });

    let data_hasher_impl: Item = syn::parse_quote! {
        impl light_sdk::light_hasher::DataHasher for CompressedAccountVariant {
            fn hash<H: light_sdk::light_hasher::Hasher>(&self) -> std::result::Result<[u8; 32], light_sdk::light_hasher::HasherError> {
                match self {
                    #(#hash_match_arms),*
                }
            }
        }
    };

    // Generate LightDiscriminator implementation
    let light_discriminator_impl: Item = syn::parse_quote! {
        impl light_sdk::LightDiscriminator for CompressedAccountVariant {
            const LIGHT_DISCRIMINATOR: [u8; 8] = [0; 8]; // This won't be used directly
            const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
        }
    };

    // Generate HasCompressionInfo implementation
    let compression_info_arms = struct_names.iter().map(|name| {
        quote! {
            Self::#name(data) => data.compression_info()
        }
    });

    let compression_info_mut_arms = struct_names.iter().map(|name| {
        quote! {
            Self::#name(data) => data.compression_info_mut()
        }
    });

    let has_compression_info_impl: Item = syn::parse_quote! {
        impl light_sdk::compressible::HasCompressionInfo for CompressedAccountVariant {
            fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
                match self {
                    #(#compression_info_arms),*
                }
            }

            fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
                match self {
                    #(#compression_info_mut_arms),*
                }
            }
        }
    };

    // Generate CompressedAccountData struct
    let compressed_account_data: ItemStruct = syn::parse_quote! {
        /// Client-side data structure for passing compressed accounts
        #[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
        pub struct CompressedAccountData {
            pub meta: light_sdk_types::instruction::account_meta::CompressedAccountMeta,
            pub data: CompressedAccountVariant,
            pub seeds: Vec<Vec<u8>>, // Seeds for PDA derivation (without bump)
        }
    };

    // Generate config-related structs and instructions
    let initialize_config_accounts: ItemStruct = syn::parse_quote! {
        #[derive(Accounts)]
        pub struct CreateCompressibleConfig<'info> {
            #[account(mut)]
            pub payer: Signer<'info>,
            /// The config PDA to be created
            #[account(
                mut,
                seeds = [b"compressible_config"],
                bump
            )]
            pub config: AccountInfo<'info>,
            /// The program's data account
            pub program_data: AccountInfo<'info>,
            /// The program's upgrade authority (must sign)
            pub authority: Signer<'info>,
            pub system_program: Program<'info, System>,
        }
    };

    let update_config_accounts: ItemStruct = syn::parse_quote! {
        #[derive(Accounts)]
        pub struct UpdateCompressibleConfig<'info> {
            #[account(
                mut,
                seeds = [b"compressible_config"],
                bump,
            )]
            pub config: AccountInfo<'info>,
            /// Must match the update authority stored in config
            pub authority: Signer<'info>,
        }
    };

    let initialize_config_fn: ItemFn = syn::parse_quote! {
        /// Create compressible config - only callable by program upgrade authority
        pub fn create_compression_config(
            ctx: Context<CreateCompressibleConfig>,
            compression_delay: u32,
            rent_recipient: Pubkey,
            address_space: Pubkey,
        ) -> Result<()> {
            light_sdk::compressible::create_compression_config_checked(
                &ctx.accounts.config.to_account_info(),
                &ctx.accounts.authority.to_account_info(),
                &ctx.accounts.program_data.to_account_info(),
                &rent_recipient,
                &address_space,
                compression_delay,
                &ctx.accounts.payer.to_account_info(),
                &ctx.accounts.system_program.to_account_info(),
                &crate::ID,
            )
            .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

            Ok(())
        }
    };

    let update_config_fn: ItemFn = syn::parse_quote! {
        /// Update compressible config - only callable by config's update authority
        pub fn update_compression_config(
            ctx: Context<UpdateCompressibleConfig>,
            new_compression_delay: Option<u32>,
            new_rent_recipient: Option<Pubkey>,
            new_address_space: Option<Pubkey>,
            new_update_authority: Option<Pubkey>,
        ) -> Result<()> {
            light_sdk::compressible::update_compression_config(
                &ctx.accounts.config.to_account_info(),
                &ctx.accounts.authority.to_account_info(),
                new_update_authority.as_ref(),
                new_rent_recipient.as_ref(),
                new_address_space.as_ref(),
                new_compression_delay,
                &crate::ID,
            )
            .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

            Ok(())
        }
    };

    // Generate the decompress_multiple_pdas accounts struct
    let decompress_accounts: ItemStruct = syn::parse_quote! {
        #[derive(Accounts)]
        pub struct DecompressMultiplePdas<'info> {
            #[account(mut)]
            pub fee_payer: Signer<'info>,
            #[account(mut)]
            pub rent_payer: Signer<'info>,
            pub system_program: Program<'info, System>,
            // Remaining accounts:
            // - First N accounts: PDA accounts to decompress into
            // - After system_accounts_offset: Light Protocol system accounts for CPI
        }
    };

    // Generate the decompress_multiple_pdas instruction
    let variant_match_arms = struct_names.iter().map(|name| {
        quote! {
            CompressedAccountVariant::#name(data) => {
                CompressedAccountVariant::#name(data)
            }
        }
    });

    let decompress_instruction: ItemFn = syn::parse_quote! {
        /// Decompresses multiple compressed PDAs of any supported account type in a single transaction
        pub fn decompress_multiple_pdas<'info>(
            ctx: Context<'_, '_, '_, 'info, DecompressMultiplePdas<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<CompressedAccountData>,
            bumps: Vec<u8>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            // Get PDA accounts from remaining accounts
            let pda_accounts_end = system_accounts_offset as usize;
            let pda_accounts = &ctx.remaining_accounts[..pda_accounts_end];

            // Validate we have matching number of PDAs, compressed accounts, and bumps
            if pda_accounts.len() != compressed_accounts.len() || pda_accounts.len() != bumps.len() {
                return err!(ErrorCode::InvalidAccountCount);
            }

            let cpi_accounts = light_sdk::cpi::CpiAccounts::new(
                &ctx.accounts.fee_payer,
                &ctx.remaining_accounts[system_accounts_offset as usize..],
                LIGHT_CPI_SIGNER,
            );

            // Convert to unified enum accounts
            let mut light_accounts = Vec::new();
            let mut pda_account_refs = Vec::new();
            let mut signer_seeds_storage = Vec::new();

            for (i, (compressed_data, bump)) in compressed_accounts.into_iter().zip(bumps.iter()).enumerate() {
                // Convert to unified enum type
                let unified_account = match compressed_data.data {
                    #(#variant_match_arms)*
                };

                let light_account = light_sdk::account::LightAccount::<'_, CompressedAccountVariant>::new_mut(
                    &crate::ID,
                    &compressed_data.meta,
                    unified_account.clone(),
                )
                .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

                // Build signer seeds based on account type
                let seeds = match &unified_account {
                    #(
                        CompressedAccountVariant::#struct_names(data) => {
                            // Get the seeds from the instruction data and append bump
                            let mut seeds = compressed_data.seeds.clone();
                            seeds.push(vec![*bump]);
                            seeds
                        }
                    ),*
                };

                signer_seeds_storage.push(seeds);
                light_accounts.push(light_account);
                pda_account_refs.push(&pda_accounts[i]);
            }

            // Convert to the format needed by the SDK
            let signer_seeds_refs: Vec<Vec<&[u8]>> = signer_seeds_storage
                .iter()
                .map(|seeds| seeds.iter().map(|s| s.as_slice()).collect())
                .collect();
            let signer_seeds_slices: Vec<&[&[u8]]> = signer_seeds_refs
                .iter()
                .map(|seeds| seeds.as_slice())
                .collect();

            // Single CPI call with unified enum type
            light_sdk::compressible::decompress_multiple_idempotent::<CompressedAccountVariant>(
                &pda_account_refs,
                light_accounts,
                &signer_seeds_slices,
                proof,
                cpi_accounts,
                &crate::ID,
                &ctx.accounts.rent_payer,
            )
            .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

            Ok(())
        }
    };

    // Generate error code enum if it doesn't exist
    let error_code: Item = syn::parse_quote! {
        #[error_code]
        pub enum ErrorCode {
            #[msg("Invalid account count: PDAs and compressed accounts must match")]
            InvalidAccountCount,
            #[msg("Rent recipient does not match config")]
            InvalidRentRecipient,
        }
    };

    // Add all generated items to the module
    content.1.push(Item::Enum(compressed_variant_enum));
    content.1.push(default_impl);
    content.1.push(data_hasher_impl);
    content.1.push(light_discriminator_impl);
    content.1.push(has_compression_info_impl);
    content.1.push(Item::Struct(compressed_account_data));
    content.1.push(Item::Struct(initialize_config_accounts));
    content.1.push(Item::Struct(update_config_accounts));
    content.1.push(Item::Fn(initialize_config_fn));
    content.1.push(Item::Fn(update_config_fn));
    content.1.push(Item::Struct(decompress_accounts));
    content.1.push(Item::Fn(decompress_instruction));
    content.1.push(error_code);

    // Generate compress instructions for each struct (NOT create instructions - those need custom logic)
    for struct_name in ident_list.idents {
        let compress_fn_name =
            format_ident!("compress_{}", struct_name.to_string().to_snake_case());
        let compress_accounts_name = format_ident!("Compress{}", struct_name);

        // Generate the compress accounts struct
        let compress_accounts_struct: ItemStruct = syn::parse_quote! {
            #[derive(Accounts)]
            pub struct #compress_accounts_name<'info> {
                #[account(mut)]
                pub user: Signer<'info>,
                #[account(
                    mut,
                    seeds = [b"user_record", user.key().as_ref()], // This should be customizable
                    bump,
                    // Add your custom constraints here
                )]
                pub pda_account: Account<'info, #struct_name>,
                pub system_program: Program<'info, System>,
                /// The global config account
                #[account(seeds = [b"compressible_config"], bump)]
                pub config: AccountInfo<'info>,
                /// Rent recipient - validated against config
                pub rent_recipient: AccountInfo<'info>,
            }
        };

        // Generate the compress instruction function
        let compress_instruction_fn: ItemFn = syn::parse_quote! {
            /// Compresses a #struct_name PDA using config values
            pub fn #compress_fn_name<'info>(
                ctx: Context<'_, '_, '_, 'info, #compress_accounts_name<'info>>,
                proof: light_sdk::instruction::ValidityProof,
                compressed_account_meta: light_sdk_types::instruction::account_meta::CompressedAccountMeta,
            ) -> Result<()> {
                // Load config from AccountInfo
                let config = light_sdk::compressible::CompressibleConfig::load_checked(
                    &ctx.accounts.config,
                    &crate::ID
                ).map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;

                // Verify rent recipient matches config
                if ctx.accounts.rent_recipient.key() != config.rent_recipient {
                    return err!(ErrorCode::InvalidRentRecipient);
                }

                let cpi_accounts = light_sdk::cpi::CpiAccounts::new(
                    &ctx.accounts.user,
                    &ctx.remaining_accounts[..],
                    LIGHT_CPI_SIGNER,
                );

                light_sdk::compressible::compress_pda::<#struct_name>(
                    &ctx.accounts.pda_account.to_account_info(),
                    &compressed_account_meta,
                    proof,
                    cpi_accounts,
                    &crate::ID,
                    &ctx.accounts.rent_recipient,
                    &config.compression_delay,
                )
                .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

                Ok(())
            }
        };

        // Add the generated items to the module (only compress, not create)
        content.1.push(Item::Struct(compress_accounts_struct));
        content.1.push(Item::Fn(compress_instruction_fn));
    }

    Ok(quote! {
        #module
    })
}
