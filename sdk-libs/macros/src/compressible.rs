use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    Error, Ident, ItemStruct, Result,
};

/// Arguments for the compressible macro (currently empty, but kept for future extensibility)
pub(crate) struct CompressibleArgs {}

impl Parse for CompressibleArgs {
    fn parse(_input: ParseStream) -> Result<Self> {
        // No arguments to parse for now
        Ok(CompressibleArgs {})
    }
}

/// Main function to process the compressible attribute
pub(crate) fn compressible(_args: CompressibleArgs, input: ItemStruct) -> Result<TokenStream> {
    let struct_name = &input.ident;

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

    // Generate only the compress module
    let compress_module = generate_compress_module(struct_name);

    Ok(quote! {
        #input

        #compress_module
    })
}

/// Generate the compress module with native and Anchor functions
fn generate_compress_module(struct_name: &Ident) -> TokenStream {
    let module_name = format_ident!("__compress_{}", struct_name.to_string().to_snake_case());
    let compress_fn_name = format_ident!("compress_{}", struct_name.to_string().to_snake_case());
    let compress_accounts_name = format_ident!("Compress{}", struct_name);

    quote! {
        // Generate the module at the crate level
        #[doc(hidden)]
        pub mod #module_name {
            use super::*;
            use light_sdk::compressible::compress_pda;
            use light_sdk::cpi::CpiAccounts;
            use light_sdk::instruction::{account_meta::CompressedAccountMeta, ValidityProof};
            use light_sdk_types::CpiAccountsConfig;
            use anchor_lang::prelude::*;

            /// Anchor function for compressing a #struct_name
            pub fn #compress_fn_name<'info>(
                ctx: Context<'_, '_, '_, 'info, #compress_accounts_name<'info>>,
                proof: ValidityProof,
                compressed_account_meta: CompressedAccountMeta,
            ) -> Result<()> {
                let config = CpiAccountsConfig::new(super::LIGHT_CPI_SIGNER);
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
                    &super::ID,
                    &ctx.accounts.rent_recipient,
                )
                .map_err(|e| ProgramError::from(e))?;
                Ok(())
            }

            #[derive(Accounts)]
            pub struct #compress_accounts_name<'info> {
                /// CHECK: The PDA to compress (unchecked)
                pub pda_account: UncheckedAccount<'info>,
                #[account(mut)]
                pub fee_payer: Signer<'info>,
                #[account(address = super::RENT_RECIPIENT)]
                /// CHECK: Validated against hardcoded RENT_RECIPIENT
                pub rent_recipient: UncheckedAccount<'info>,
            }
        }

        // Re-export for use inside the program module
        pub use #module_name::{#compress_fn_name, #compress_accounts_name};
    }
}
