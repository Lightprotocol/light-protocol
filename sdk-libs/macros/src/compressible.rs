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

    // Generate CompressionTiming implementation
    let last_written_slot_arms = struct_names.iter().map(|name| {
        quote! {
            Self::#name(data) => data.last_written_slot()
        }
    });

    let compression_delay_arms = struct_names.iter().map(|name| {
        quote! {
            Self::#name(data) => data.compression_delay()
        }
    });

    let set_last_written_slot_arms = struct_names.iter().map(|name| {
        quote! {
            Self::#name(data) => data.set_last_written_slot(slot)
        }
    });

    let pda_timing_impl: Item = syn::parse_quote! {
        impl light_sdk::compressible::CompressionTiming for CompressedAccountVariant {
            fn last_written_slot(&self) -> u64 {
                match self {
                    #(#last_written_slot_arms),*
                }
            }

            fn compression_delay(&self) -> u64 {
                match self {
                    #(#compression_delay_arms),*
                }
            }

            fn set_last_written_slot(&mut self, slot: u64) {
                match self {
                    #(#set_last_written_slot_arms),*
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
            proof: ValidityProof,
            compressed_accounts: Vec<CompressedAccountData>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            // Get PDA accounts from remaining accounts
            let pda_accounts_end = system_accounts_offset as usize;
            let pda_accounts = &ctx.remaining_accounts[..pda_accounts_end];

            // Validate we have matching number of PDAs and compressed accounts
            if pda_accounts.len() != compressed_accounts.len() {
                return err!(ErrorCode::InvalidAccountCount);
            }

            // Set up CPI accounts
            let cpi_accounts = CpiAccounts::new(
                &ctx.accounts.fee_payer,
                &ctx.remaining_accounts[system_accounts_offset as usize..],
                LIGHT_CPI_SIGNER,
            );

            // Convert to unified enum accounts
            let mut light_accounts = Vec::new();
            let mut pda_account_refs = Vec::new();

            for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
                // Convert to unified enum type
                let unified_account = match compressed_data.data {
                    #(#variant_match_arms)*
                };

                let light_account = light_sdk::account::LightAccount::<'_, CompressedAccountVariant>::new_mut(
                    &crate::ID,
                    &compressed_data.meta,
                    unified_account,
                )
                .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

                light_accounts.push(light_account);
                pda_account_refs.push(&pda_accounts[i]);
            }

            // Single CPI call with unified enum type
            light_sdk::compressible::decompress_multiple_idempotent::<CompressedAccountVariant>(
                &pda_account_refs,
                light_accounts,
                proof,
                cpi_accounts,
                &crate::ID,
                &ctx.accounts.rent_payer,
                &ctx.accounts.system_program.to_account_info(),
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
        }
    };

    // Add all generated items to the module
    content.1.push(Item::Enum(compressed_variant_enum));
    content.1.push(default_impl);
    content.1.push(data_hasher_impl);
    content.1.push(light_discriminator_impl);
    content.1.push(pda_timing_impl);
    content.1.push(Item::Struct(compressed_account_data));
    content.1.push(Item::Struct(decompress_accounts));
    content.1.push(Item::Fn(decompress_instruction));
    content.1.push(error_code);

    // Generate compress instructions for each struct
    for struct_name in ident_list.idents {
        let compress_fn_name =
            format_ident!("compress_{}", struct_name.to_string().to_snake_case());
        let compress_accounts_name = format_ident!("Compress{}", struct_name);

        // Generate the accounts struct
        let accounts_struct: ItemStruct = syn::parse_quote! {
            #[derive(Accounts)]
            pub struct #compress_accounts_name<'info> {
                /// CHECK: The PDA to compress (unchecked)
                pub pda_account: UncheckedAccount<'info>,
                #[account(mut)]
                pub fee_payer: Signer<'info>,
                #[account(address = RENT_RECIPIENT)]
                /// CHECK: Validated against hardcoded RENT_RECIPIENT
                pub rent_recipient: UncheckedAccount<'info>,
            }
        };

        // Generate the instruction function
        let instruction_fn: ItemFn = syn::parse_quote! {
            /// Compresses a #struct_name PDA
            pub fn #compress_fn_name<'info>(
                ctx: Context<'_, '_, '_, 'info, #compress_accounts_name<'info>>,
                proof: ValidityProof,
                compressed_account_meta: light_sdk_types::instruction::account_meta::CompressedAccountMeta,
            ) -> Result<()> {
                let config = CpiAccountsConfig::new(LIGHT_CPI_SIGNER);
                let cpi_accounts = CpiAccounts::new_with_config(
                    &ctx.accounts.fee_payer,
                    &ctx.remaining_accounts[..],
                    config,
                );

                light_sdk::compressible::compress_pda::<#struct_name>(
                    &ctx.accounts.pda_account,
                    &compressed_account_meta,
                    proof,
                    cpi_accounts,
                    &crate::ID,
                    &ctx.accounts.rent_recipient,
                )
                .map_err(|e| ProgramError::from(e))?;

                Ok(())
            }
        };

        // Add the generated items to the module
        content.1.push(Item::Struct(accounts_struct));
        content.1.push(Item::Fn(instruction_fn));
    }

    Ok(quote! {
        #module
    })
}
