use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Ident, Item, ItemFn, ItemMod, ItemStruct, Result, Token,
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

    // Generate instructions for each struct
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
