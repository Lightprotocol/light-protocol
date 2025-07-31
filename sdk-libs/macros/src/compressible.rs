use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Ident, Item, ItemEnum, ItemFn, ItemMod, ItemStruct, Result, Token,
};

/// Parse a comma-separated list of identifiers
#[derive(Clone)]
enum CompressibleType {
    Regular(Ident),
}

struct CompressibleTypeList {
    types: Punctuated<CompressibleType, Token![,]>,
}

impl Parse for CompressibleType {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident: Ident = input.parse()?;
        Ok(CompressibleType::Regular(ident))
    }
}

impl Parse for CompressibleTypeList {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(CompressibleTypeList {
            types: Punctuated::parse_terminated(input)?,
        })
    }
}

/// Generate compress instructions for the specified account types (Anchor version)
pub(crate) fn add_compressible_instructions(
    args: TokenStream,
    mut module: ItemMod,
) -> Result<TokenStream> {
    let type_list = syn::parse2::<CompressibleTypeList>(args)?;

    // Check if module has content
    if module.content.is_none() {
        return Err(syn::Error::new_spanned(&module, "Module must have a body"));
    }

    // Collect all struct names
    let mut all_struct_names = Vec::new();

    for compressible_type in &type_list.types {
        match compressible_type {
            CompressibleType::Regular(ident) => {
                all_struct_names.push(ident.clone());
            }
        }
    }

    // Note: All account types must implement CompressAs trait

    // Get the module content
    let content = module.content.as_mut().unwrap();

    // Collect all struct names for the enum
    let struct_names: Vec<_> = all_struct_names.iter().cloned().collect();

    // Generate the CompressedAccountVariant enum
    let enum_variants = struct_names.iter().map(|name| {
        quote! { #name(#name) }
    });

    let compressed_account_variant_enum: ItemEnum = syn::parse_quote! {
        #[derive(Clone, Debug, light_sdk::AnchorSerialize, light_sdk::AnchorDeserialize)]
        pub enum CompressedAccountVariant {
            #(#enum_variants),*
        }
    };

    // Generate Default implementation for the enum
    if struct_names.is_empty() {
        return Err(syn::Error::new_spanned(
            &module,
            "At least one account struct must be specified",
        ));
    }

    let first_struct = struct_names.first().expect("At least one struct required");
    let default_impl: Item = syn::parse_quote! {
        impl Default for CompressedAccountVariant {
            fn default() -> Self {
                CompressedAccountVariant::#first_struct(Default::default())
            }
        }
    };

    // Generate DataHasher implementation for the enum
    let hash_match_arms = struct_names.iter().map(|name| {
        quote! {
            CompressedAccountVariant::#name(data) => data.hash::<H>()
        }
    });

    let data_hasher_impl: Item = syn::parse_quote! {
        impl light_hasher::DataHasher for CompressedAccountVariant {
            fn hash<H: light_hasher::Hasher>(&self) -> std::result::Result<[u8; 32], light_hasher::errors::HasherError> {
                match self {
                    #(#hash_match_arms),*
                }
            }
        }
    };

    // Generate LightDiscriminator implementation for the enum
    let light_discriminator_impl: Item = syn::parse_quote! {
        impl light_sdk::LightDiscriminator for CompressedAccountVariant {
            const LIGHT_DISCRIMINATOR: [u8; 8] = [0; 8]; // This won't be used directly
            const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
        }
    };

    // Generate HasCompressionInfo implementation for the enum
    let has_compression_info_impl: Item = syn::parse_quote! {
        impl light_sdk::compressible::HasCompressionInfo for CompressedAccountVariant {
            fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
                match self {
                    #(CompressedAccountVariant::#struct_names(data) => data.compression_info()),*
                }
            }

            fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
                match self {
                    #(CompressedAccountVariant::#struct_names(data) => data.compression_info_mut()),*
                }
            }

            fn compression_info_mut_opt(&mut self) -> &mut Option<light_sdk::compressible::CompressionInfo> {
                match self {
                    #(CompressedAccountVariant::#struct_names(data) => data.compression_info_mut_opt()),*
                }
            }

            fn set_compression_info_none(&mut self) {
                match self {
                    #(CompressedAccountVariant::#struct_names(data) => data.set_compression_info_none()),*
                }
            }
        }
    };

    // Generate Size implementation for the enum
    let size_match_arms = struct_names.iter().map(|name| {
        quote! {
            CompressedAccountVariant::#name(data) => data.size()
        }
    });

    let size_impl: Item = syn::parse_quote! {
        impl light_sdk::Size for CompressedAccountVariant {
            fn size(&self) -> usize {
                match self {
                    #(#size_match_arms),*
                }
            }
        }
    };

    // Generate the CompressedAccountData struct
    let compressed_account_data: ItemStruct = syn::parse_quote! {
        #[derive(Clone, Debug, light_sdk::AnchorDeserialize, light_sdk::AnchorSerialize)]
        pub struct CompressedAccountData {
            pub meta: light_sdk_types::instruction::account_meta::CompressedAccountMeta,
            pub data: CompressedAccountVariant,
            pub seeds: Vec<Vec<u8>>, // Seeds for PDA derivation (without bump)
        }
    };

    // Generate config-related structs and instructions
    let initialize_config_accounts: ItemStruct = syn::parse_quote! {
        #[derive(Accounts)]
        pub struct InitializeCompressionConfig<'info> {
            #[account(mut)]
            pub payer: Signer<'info>,
            /// The config PDA to be created
            /// CHECK: Config PDA is checked by the SDK
            #[account(mut)]
            pub config: AccountInfo<'info>,
            /// The program's data account
            /// CHECK: Program data account is validated by the SDK
            pub program_data: AccountInfo<'info>,
            /// The program's upgrade authority (must sign)
            pub authority: Signer<'info>,
            pub system_program: Program<'info, System>,
        }
    };

    // Generate the update_compression_config accounts struct
    let update_config_accounts: ItemStruct = syn::parse_quote! {
        #[derive(Accounts)]
        pub struct UpdateCompressionConfig<'info> {
            /// CHECK: Config is checked by the SDK's load_checked method
            #[account(mut)]
            pub config: AccountInfo<'info>,
            /// Must match the update authority stored in config
            pub authority: Signer<'info>,
        }
    };

    let initialize_compression_config_fn: ItemFn = syn::parse_quote! {
        /// Create compressible config - only callable by program upgrade authority
        pub fn initialize_compression_config(
            ctx: Context<InitializeCompressionConfig>,
            compression_delay: u32,
            rent_recipient: Pubkey,
            address_space: Vec<Pubkey>,
            config_bump: Option<u8>,
        ) -> anchor_lang::Result<()> {
            let config_bump = config_bump.unwrap_or(0);
            light_sdk::compressible::process_initialize_compression_config_checked(
                &ctx.accounts.config.to_account_info(),
                &ctx.accounts.authority.to_account_info(),
                &ctx.accounts.program_data.to_account_info(),
                &rent_recipient,
                address_space,
                compression_delay,
                config_bump,
                &ctx.accounts.payer.to_account_info(),
                &ctx.accounts.system_program.to_account_info(),
                &super::ID,
            )?;

            Ok(())
        }
    };

    let update_compression_config_fn: ItemFn = syn::parse_quote! {
        /// Update compressible config - only callable by config's update authority
        pub fn update_compression_config(
            ctx: Context<UpdateCompressionConfig>,
            new_compression_delay: Option<u32>,
            new_rent_recipient: Option<Pubkey>,
            new_address_space: Option<Vec<Pubkey>>,
            new_update_authority: Option<Pubkey>,
        ) -> anchor_lang::Result<()> {
            light_sdk::compressible::process_update_compression_config(
                &ctx.accounts.config.to_account_info(),
                &ctx.accounts.authority.to_account_info(),
                new_update_authority.as_ref(),
                new_rent_recipient.as_ref(),
                new_address_space,
                new_compression_delay,
                &super::ID,
            )?;

            Ok(())
        }
    };

    // Generate the decompress_accounts_idempotent accounts struct
    let decompress_accounts: ItemStruct = syn::parse_quote! {
        #[derive(Accounts)]
        pub struct DecompressAccountsIdempotent<'info> {
            #[account(mut)]
            pub fee_payer: Signer<'info>,
            /// UNCHECKED: Anyone can pay to init.
            #[account(mut)]
            pub rent_payer: Signer<'info>,
            /// The global config account
            /// CHECK: load_checked.
            pub config: AccountInfo<'info>,
            // Remaining accounts:
            // - First N accounts: PDA accounts to decompress into
            // - After system_accounts_offset: Light Protocol system accounts for CPI
        }
    };

    // Generate the decompress_accounts_idempotent instruction
    let decompress_instruction: ItemFn = syn::parse_quote! {
        /// Decompresses multiple compressed PDAs of any supported account type in a single transaction
        pub fn decompress_accounts_idempotent<'info>(
            ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<CompressedAccountData>,
            bumps: Vec<u8>,
            system_accounts_offset: u8,
        ) -> anchor_lang::Result<()> {
            // Get PDA accounts from remaining accounts
            let pda_accounts_end = system_accounts_offset as usize;
            let solana_accounts = &ctx.remaining_accounts[..pda_accounts_end];

            // Validate we have matching number of PDAs, compressed accounts, and bumps
            if solana_accounts.len() != compressed_accounts.len() || solana_accounts.len() != bumps.len() {
                return err!(ErrorCode::InvalidAccountCount);
            }

            let cpi_accounts = light_sdk::cpi::CpiAccounts::new(
                &ctx.accounts.fee_payer,
                &ctx.remaining_accounts[system_accounts_offset as usize..],
                LIGHT_CPI_SIGNER,
            );

            // Get address space from config checked.
            let config = light_sdk::compressible::CompressibleConfig::load_checked(&ctx.accounts.config, &super::ID)?;
            let address_space = config.address_space[0];

            let mut all_compressed_infos = Vec::with_capacity(compressed_accounts.len());

            for (i, (compressed_data, &bump)) in compressed_accounts
                .into_iter()
                .zip(bumps.iter())
                .enumerate()
            {
                let bump_slice = [bump];

                match compressed_data.data {
                    #(
                        CompressedAccountVariant::#struct_names(data) => {
                            let mut seeds_refs = Vec::with_capacity(compressed_data.seeds.len() + 1);
                            for seed in &compressed_data.seeds {
                                seeds_refs.push(seed.as_slice());
                            }
                            seeds_refs.push(&bump_slice);

                            // Create LightAccount with correct discriminator
                            let light_account = light_sdk::account::sha::LightAccount::<'_, #struct_names>::new_mut(
                                &super::ID,
                                &compressed_data.meta,
                                data,
                            )?;

                            // Process this single account
                            let compressed_infos = light_sdk::compressible::prepare_accounts_for_decompress_idempotent::<#struct_names>(
                                &[&solana_accounts[i]],
                                vec![light_account],
                                &[seeds_refs.as_slice()],
                                &cpi_accounts,
                                &ctx.accounts.rent_payer,
                                address_space,
                            )?;

                            all_compressed_infos.extend(compressed_infos);
                        }
                    ),*
                }
            }

            if all_compressed_infos.is_empty() {
                msg!("No compressed accounts to decompress");
            } else {
                let cpi_inputs = light_sdk::cpi::CpiInputs::new(proof, all_compressed_infos);
                cpi_inputs.invoke_light_system_program(cpi_accounts)?;
            }

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
    content.1.push(Item::Enum(compressed_account_variant_enum));
    content.1.push(default_impl);
    content.1.push(data_hasher_impl);
    content.1.push(light_discriminator_impl);
    content.1.push(has_compression_info_impl);
    content.1.push(size_impl);
    content.1.push(Item::Struct(compressed_account_data));
    content.1.push(Item::Struct(initialize_config_accounts));
    content.1.push(Item::Struct(update_config_accounts));
    content.1.push(Item::Fn(initialize_compression_config_fn));
    content.1.push(Item::Fn(update_compression_config_fn));
    content.1.push(Item::Struct(decompress_accounts));
    content.1.push(Item::Fn(decompress_instruction));
    content.1.push(error_code);

    // Generate compress instructions for each struct
    for compressible_type in type_list.types {
        let struct_name = match compressible_type {
            CompressibleType::Regular(ident) => ident,
        };

        let compress_fn_name =
            format_ident!("compress_{}", struct_name.to_string().to_snake_case());
        let compress_accounts_name = format_ident!("Compress{}", struct_name);

        // Generate the compress accounts struct - generic without seeds constraints
        let compress_accounts_struct: ItemStruct = syn::parse_quote! {
            #[derive(Accounts)]
            pub struct #compress_accounts_name<'info> {
                #[account(mut)]
                pub user: Signer<'info>,
                #[account(mut)]
                pub pda_to_compress: Account<'info, #struct_name>,
                /// The global config account
                /// CHECK: Config is validated by the SDK's load_checked method
                pub config: AccountInfo<'info>,
                /// Rent recipient - must match config
                /// CHECK: Rent recipient is validated against the config
                #[account(mut)]
                pub rent_recipient: AccountInfo<'info>,
            }
        };

        // Add the compress accounts struct
        content.1.push(Item::Struct(compress_accounts_struct));

        // Generate compress instruction that uses CompressAs trait
        let compress_instruction_fn: ItemFn = syn::parse_quote! {
            /// Compresses a #struct_name PDA using the CompressAs trait implementation.
            /// The account type must implement CompressAs to specify compression behavior.
            /// For simple cases, implement CompressAs with type Output = Self and return self.clone().
            /// For custom compression, you can reset specific fields or use a different output type.
            pub fn #compress_fn_name<'info>(
                ctx: Context<'_, '_, '_, 'info, #compress_accounts_name<'info>>,
                proof: light_sdk::instruction::ValidityProof,
                compressed_account_meta: light_sdk_types::instruction::account_meta::CompressedAccountMeta,
            ) -> anchor_lang::Result<()> {
                // Load config from AccountInfo
                let config = light_sdk::compressible::CompressibleConfig::load_checked(
                    &ctx.accounts.config,
                    &super::ID
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

                light_sdk::compressible::compress_account::<#struct_name>(
                    &mut ctx.accounts.pda_to_compress,
                    &compressed_account_meta,
                    proof,
                    cpi_accounts,
                    &ctx.accounts.rent_recipient,
                    &config.compression_delay,
                )
                .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

                Ok(())
            }
        };

        content.1.push(Item::Fn(compress_instruction_fn));
    }

    Ok(quote! {
        #module
    })
}

/// Generates HasCompressionInfo trait implementation for a struct with compression_info field
pub fn derive_has_compression_info(input: syn::ItemStruct) -> Result<TokenStream> {
    let struct_name = input.ident.clone();

    // Find the compression_info field
    let compression_info_field = match &input.fields {
        syn::Fields::Named(fields) => fields.named.iter().find(|field| {
            field
                .ident
                .as_ref()
                .map(|ident| ident == "compression_info")
                .unwrap_or(false)
        }),
        _ => {
            return Err(syn::Error::new_spanned(
                &struct_name,
                "HasCompressionInfo can only be derived for structs with named fields",
            ))
        }
    };

    let _compression_info_field = compression_info_field.ok_or_else(|| {
        syn::Error::new_spanned(
            &struct_name,
            "HasCompressionInfo requires a field named 'compression_info' of type Option<CompressionInfo>"
        )
    })?;

    // Validate that the field is Option<CompressionInfo>
    // For now, we'll assume it's correct and let the compiler catch type errors

    let has_compression_info_impl = quote! {
        impl light_sdk::compressible::HasCompressionInfo for #struct_name {
            fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
                self.compression_info
                    .as_ref()
                    .expect("CompressionInfo must be Some on-chain")
            }

            fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
                self.compression_info
                    .as_mut()
                    .expect("CompressionInfo must be Some on-chain")
            }

            fn compression_info_mut_opt(&mut self) -> &mut Option<light_sdk::compressible::CompressionInfo> {
                &mut self.compression_info
            }

            fn set_compression_info_none(&mut self) {
                self.compression_info = None;
            }
        }
    };

    Ok(has_compression_info_impl)
}
