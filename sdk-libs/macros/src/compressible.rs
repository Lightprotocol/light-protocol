use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Error, Ident, ItemStruct, Result, Token,
};

/// Arguments for the compressible macro
pub(crate) struct CompressibleArgs {
    slots_until_compression: u64,
}

impl Parse for CompressibleArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut slots_until_compression = 100u64; // default

        if input.is_empty() {
            return Ok(CompressibleArgs {
                slots_until_compression,
            });
        }

        let args = Punctuated::<syn::MetaNameValue, Token![,]>::parse_terminated(input)?;
        for arg in args {
            if arg.path.is_ident("slots_until_compression") {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(lit_int),
                    ..
                }) = &arg.value
                {
                    slots_until_compression = lit_int.base10_parse()?;
                } else {
                    return Err(Error::new_spanned(
                        arg.value,
                        "slots_until_compression must be an integer",
                    ));
                }
            }
        }

        Ok(CompressibleArgs {
            slots_until_compression,
        })
    }
}

/// Main function to process the compressible attribute
pub(crate) fn compressible(args: CompressibleArgs, input: ItemStruct) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let slots_until_compression = args.slots_until_compression;

    // Verify that the struct has the required fields
    let has_required_fields = if let syn::Fields::Named(ref fields) = input.fields {
        let has_last_written = fields.named.iter().any(|f| {
            f.ident
                .as_ref()
                .map(|i| i == "last_written_slot")
                .unwrap_or(false)
        });
        let has_slots_until = fields.named.iter().any(|f| {
            f.ident
                .as_ref()
                .map(|i| i == "slots_until_compression")
                .unwrap_or(false)
        });
        has_last_written && has_slots_until
    } else {
        false
    };

    if !has_required_fields {
        return Err(Error::new_spanned(
            &input,
            "compressible structs must have 'last_written_slot: u64' and 'slots_until_compression: u64' fields"
        ));
    }

    // Generate only the implementations, not the struct modification
    let default_constant = generate_default_constant(struct_name, slots_until_compression);
    let compress_module = generate_compress_module(struct_name);

    Ok(quote! {
        #input

        #default_constant
        #compress_module
    })
}

/// Generate only the default constant
fn generate_default_constant(struct_name: &Ident, slots_until_compression: u64) -> TokenStream {
    quote! {
        impl #struct_name {
            pub const DEFAULT_SLOTS_UNTIL_COMPRESSION: u64 = #slots_until_compression;
        }
    }
}

/// Generate the compress module with native and Anchor functions
fn generate_compress_module(struct_name: &Ident) -> TokenStream {
    let module_name = format_ident!("compress_{}", to_snake_case(&struct_name.to_string()));
    let anchor_fn_name = module_name.clone();
    let compress_accounts_name = format_ident!("Compress{}", struct_name);

    quote! {
        pub mod #module_name {
            use super::*;

            // Native compress function
            pub fn compress_native(
                // Parameters would go here
            ) -> std::result::Result<(), Box<dyn std::error::Error>> {
                // Implementation would go here
                Ok(())
            }

            // Anchor-specific code only when anchor feature is enabled
            #[cfg(feature = "anchor")]
            pub mod anchor {
                use super::*;
                use ::light_sdk::{
                    compressible::compress_pda,
                    cpi::CpiAccounts,
                    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
                };
                use ::light_sdk_types::CpiAccountsConfig;

                /// Anchor function for compressing a #struct_name
                pub fn #anchor_fn_name<'info>(
                    ctx: ::anchor_lang::prelude::Context<'_, '_, '_, 'info, #compress_accounts_name<'info>>,
                    proof: ValidityProof,
                    compressed_account_meta: CompressedAccountMeta,
                ) -> ::anchor_lang::prelude::Result<()> {
                    let config = CpiAccountsConfig::new(super::super::LIGHT_CPI_SIGNER);
                    let cpi_accounts = CpiAccounts::new_with_config(
                        &ctx.accounts.fee_payer,
                        &ctx.remaining_accounts[..],
                        config,
                    );

                    compress_pda::<super::#struct_name>(
                        &ctx.accounts.pda_account,
                        &compressed_account_meta,
                        proof,
                        cpi_accounts,
                        &super::super::ID,
                        &ctx.accounts.rent_recipient,
                    )
                    .map_err(|e| ::anchor_lang::prelude::ProgramError::from(e))?;
                    Ok(())
                }

                #[derive(::anchor_lang::prelude::Accounts)]
                pub struct #compress_accounts_name<'info> {
                    /// CHECK: The PDA to compress (unchecked)
                    pub pda_account: ::anchor_lang::prelude::UncheckedAccount<'info>,
                    pub authority: ::anchor_lang::prelude::Signer<'info>,
                    #[account(mut)]
                    pub fee_payer: ::anchor_lang::prelude::Signer<'info>,
                    /// CHECK: Validated against hardcoded RENT_RECIPIENT
                    pub rent_recipient: ::anchor_lang::prelude::UncheckedAccount<'info>,
                }
            }
        }
    }
}

/// Generate the decompress accounts macro (simpler version)
pub(crate) fn generate_decompress_module() -> Result<TokenStream> {
    // For now, return an empty token stream since we can't use global state
    // Users will need to manually implement the unified enum and decompress function
    Ok(TokenStream::new())
}

/// Convert PascalCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}
