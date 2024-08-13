use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, FieldsNamed, Ident, Result};

pub(crate) fn process_light_traits(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;

    let trait_impls = match input.data {
        Data::Struct(data_struct) => match data_struct.fields {
            Fields::Named(fields) => process_fields_and_attributes(name, fields),
            _ => quote! {
                compile_error!("Error: Expected named fields but found unnamed or no fields.");
            },
        },
        _ => quote! {},
    };

    let expanded = quote! {
        #trait_impls
    };

    Ok(expanded)
}

fn process_fields_and_attributes(name: &Ident, fields: FieldsNamed) -> TokenStream {
    let mut self_program_field = None;
    let mut fee_payer_field = None;
    let mut authority_field = None;
    let mut light_system_program_field = None;
    let mut cpi_context_account_field = None;

    // base impl
    let mut registered_program_pda_field = None;
    let mut noop_program_field = None;
    let mut account_compression_authority_field = None;
    let mut account_compression_program_field = None;
    let mut system_program_field = None;

    let compressed_sol_pda_field = fields
        .named
        .iter()
        .find_map(|f| {
            if f.ident
                .as_ref()
                .map(|id| id == "compressed_sol_pda")
                .unwrap_or(false)
            {
                Some(quote! { self.#f.ident.as_ref() })
            } else {
                None
            }
        })
        .unwrap_or(quote! { None });

    let compression_recipient_field = fields
        .named
        .iter()
        .find_map(|f| {
            if f.ident
                .as_ref()
                .map(|id| id == "compression_recipient")
                .unwrap_or(false)
            {
                Some(quote! { self.#f.ident.as_ref() })
            } else {
                None
            }
        })
        .unwrap_or(quote! { None });

    for f in fields.named.iter() {
        for attr in &f.attrs {
            if attr.path.is_ident("self_program") {
                self_program_field = Some(f.ident.as_ref().unwrap());
            }
            if attr.path.is_ident("fee_payer") {
                fee_payer_field = Some(f.ident.as_ref().unwrap());
            }
            if attr.path.is_ident("authority") {
                authority_field = Some(f.ident.as_ref().unwrap());
            }
            if attr.path.is_ident("cpi_context") {
                cpi_context_account_field = Some(f.ident.as_ref().unwrap());
            }
        }
        if f.ident
            .as_ref()
            .map(|id| id == "light_system_program")
            .unwrap_or(false)
        {
            light_system_program_field = Some(f.ident.as_ref().unwrap());
        }
        if f.ident
            .as_ref()
            .map(|id| id == "registered_program_pda")
            .unwrap_or(false)
        {
            registered_program_pda_field = Some(f.ident.as_ref().unwrap());
        }
        if f.ident
            .as_ref()
            .map(|id| id == "noop_program")
            .unwrap_or(false)
        {
            noop_program_field = Some(f.ident.as_ref().unwrap());
        }
        if f.ident
            .as_ref()
            .map(|id| id == "account_compression_authority")
            .unwrap_or(false)
        {
            account_compression_authority_field = Some(f.ident.as_ref().unwrap());
        }
        if f.ident
            .as_ref()
            .map(|id| id == "account_compression_program")
            .unwrap_or(false)
        {
            account_compression_program_field = Some(f.ident.as_ref().unwrap());
        }
        if f.ident
            .as_ref()
            .map(|id| id == "system_program")
            .unwrap_or(false)
        {
            system_program_field = Some(f.ident.as_ref().unwrap());
        }
    }

    // optional: compressed_sol_pda, compression_recipient, cpi_context_account
    let missing_required_fields = [
        if light_system_program_field.is_none() {
            "light_system_program"
        } else {
            ""
        },
        if registered_program_pda_field.is_none() {
            "registered_program_pda"
        } else {
            ""
        },
        if noop_program_field.is_none() {
            "noop_program"
        } else {
            ""
        },
        if account_compression_authority_field.is_none() {
            "account_compression_authority"
        } else {
            ""
        },
        if account_compression_program_field.is_none() {
            "account_compression_program"
        } else {
            ""
        },
        if system_program_field.is_none() {
            "system_program"
        } else {
            ""
        },
    ]
    .iter()
    .filter(|&field| !field.is_empty())
    .cloned()
    .collect::<Vec<_>>();

    let missing_required_attributes = [
        if self_program_field.is_none() {
            "self_program"
        } else {
            ""
        },
        if fee_payer_field.is_none() {
            "fee_payer"
        } else {
            ""
        },
        if authority_field.is_none() {
            "authority"
        } else {
            ""
        },
    ]
    .iter()
    .filter(|&attr| !attr.is_empty())
    .cloned()
    .collect::<Vec<_>>();

    if !missing_required_fields.is_empty() || !missing_required_attributes.is_empty() {
        let error_message = format!(
            "Error: Missing required fields: [{}], Missing required attributes: [{}]",
            missing_required_fields.join(", "),
            missing_required_attributes.join(", ")
        );
        quote! {
            compile_error!(#error_message);
        }
    } else {
        let base_impls = quote! {
            impl<'info> ::light_sdk::traits::InvokeCpiAccounts<'info> for #name<'info> {
                fn get_invoking_program(&self) -> &AccountInfo<'info> {
                    &self.#self_program_field
                }
            }
            impl<'info> ::light_sdk::traits::SignerAccounts<'info> for #name<'info> {
                fn get_fee_payer(&self) -> &::anchor_lang::prelude::Signer<'info> {
                    &self.#fee_payer_field
                }
                fn get_authority(&self) -> &::anchor_lang::prelude::AccountInfo<'info> {
                    &self.#authority_field
                }
            }
            impl<'info> ::light_sdk::traits::LightSystemAccount<'info> for #name<'info> {
                fn get_light_system_program(&self) -> &::anchor_lang::prelude::Program<
                    'info,
                    ::light_system_program::program::LightSystemProgram
                > {
                    &self.#light_system_program_field
                }
            }
        };
        let invoke_accounts_impl = quote! {
            impl<'info> ::light_sdk::traits::InvokeAccounts<'info> for #name<'info> {
                fn get_registered_program_pda(&self) -> &::anchor_lang::prelude::Account<
                    'info,
                    ::account_compression::RegisteredProgram
                > {
                    &self.#registered_program_pda_field
                }
                fn get_noop_program(&self) -> &::anchor_lang::prelude::AccountInfo<'info> {
                    &self.#noop_program_field
                }
                fn get_account_compression_authority(&self) -> &::anchor_lang::prelude::AccountInfo<'info> {
                    &self.#account_compression_authority_field
                }
                fn get_account_compression_program(&self) -> &::anchor_lang::prelude::Program<
                    'info,
                    ::account_compression::program::AccountCompression
                > {
                    &self.#account_compression_program_field
                }
                fn get_system_program(&self) -> &::anchor_lang::prelude::Program<'info, System> {
                    &self.#system_program_field
                }
                fn get_compressed_sol_pda(&self) -> Option<&::anchor_lang::prelude::AccountInfo<'info>> {
                    #compressed_sol_pda_field
                }
                fn get_compression_recipient(&self) -> Option<&::anchor_lang::prelude::AccountInfo<'info>> {
                    #compression_recipient_field
                }
            }
        };
        if cpi_context_account_field.is_none() {
            quote! {
                #base_impls
                #invoke_accounts_impl
                impl<'info> ::light_sdk::traits::InvokeCpiContextAccount<'info> for #name<'info> {
                    fn get_cpi_context_account(&self) -> Option<
                        &::anchor_lang::prelude::Account<
                            'info,
                            ::light_system_program::invoke_cpi::account::CpiContextAccount
                        >
                    > {
                        None
                    }
                }
            }
        } else {
            quote! {
                #base_impls
                #invoke_accounts_impl
                impl<'info> ::light_sdk::traits::InvokeCpiContextAccount<'info> for #name<'info> {
                    fn get_cpi_context_account(&self) -> Option<
                        &::anchor_lang::prelude::Account<
                            'info,
                            ::light_system_program::invoke_cpi::account::CpiContextAccount
                        >
                    > {
                        Some(&self.#cpi_context_account_field)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, DeriveInput, FieldsNamed};

    #[test]
    fn test_process_light_traits() {
        let input: DeriveInput = parse_quote! {
            struct TestStruct {
                #[self_program]
                pub my_program: Program<'info, MyProgram>,
                #[fee_payer]
                pub payer: Signer<'info>,
                #[authority]
                pub user: AccountInfo<'info>,
                pub light_system_program: Program<'info, LightSystemProgram>,
                pub registered_program_pda: Account<'info, RegisteredProgram>,
                pub noop_program: AccountInfo<'info>,
                pub account_compression_authority: AccountInfo<'info>,
                pub account_compression_program: Program<'info, AccountCompression>,
                pub system_program: Program<'info, System>,
            }
        };

        let output = process_light_traits(input).unwrap();
        let output_string = output.to_string();

        assert!(output_string.contains("InvokeCpiAccounts"));
        assert!(output_string.contains("SignerAccounts"));
        assert!(output_string.contains("LightSystemAccount"));
        assert!(output_string.contains("InvokeAccounts"));
        assert!(output_string.contains("InvokeCpiContextAccount"));
    }

    #[test]
    fn test_process_fields_and_attributes() {
        let fields: FieldsNamed = parse_quote! {
            {
                #[self_program]
                pub my_program: Program<'info, MyProgram>,
                #[fee_payer]
                pub payer: Signer<'info>,
                #[authority]
                pub user: AccountInfo<'info>,
                pub light_system_program: Program<'info, LightSystemProgram>,
                pub registered_program_pda: Account<'info, RegisteredProgram>,
                pub noop_program: AccountInfo<'info>,
                pub account_compression_authority: AccountInfo<'info>,
                pub account_compression_program: Program<'info, AccountCompression>,
                pub system_program: Program<'info, System>,
            }
        };

        let name = syn::Ident::new("TestStruct", proc_macro2::Span::call_site());
        let output = process_fields_and_attributes(&name, fields);
        let output_string = output.to_string();

        assert!(output_string.contains("InvokeCpiAccounts"));
        assert!(output_string.contains("SignerAccounts"));
        assert!(output_string.contains("LightSystemAccount"));
        assert!(output_string.contains("InvokeAccounts"));
        assert!(output_string.contains("InvokeCpiContextAccount"));
    }

    #[test]
    fn test_process_light_traits_missing_fields() {
        let input: DeriveInput = parse_quote! {
            struct TestStruct {
                #[self_program]
                pub my_program: Program<'info, MyProgram>,
                #[fee_payer]
                pub payer: Signer<'info>,
                #[authority]
                pub user: AccountInfo<'info>,
                // Missing required fields
            }
        };

        let result = process_light_traits(input);
        let output_string = result.unwrap().to_string();

        assert!(output_string.contains("compile_error"));
        assert!(output_string.contains("Error: Missing required fields: [light_system_program, registered_program_pda, noop_program, account_compression_authority, account_compression_program, system_program], Missing required attributes: []"));
    }

    #[test]
    fn test_process_light_traits_missing_attributes() {
        let input: DeriveInput = parse_quote! {
            struct TestStruct {
                pub my_program: Program<'info, MyProgram>, // Missing #[self_program]
                pub payer: Signer<'info>, // Missing #[fee_payer]
                pub user: AccountInfo<'info>, // Missing #[authority]
                pub light_system_program: Program<'info, LightSystemProgram>,
                pub registered_program_pda: Account<'info, RegisteredProgram>,
                pub noop_program: AccountInfo<'info>,
                pub account_compression_authority: AccountInfo<'info>,
                pub account_compression_program: Program<'info, AccountCompression>,
                pub system_program: Program<'info, System>,
            }
        };

        let result = process_light_traits(input);
        let output_string = result.unwrap().to_string();
        assert!(output_string.contains("compile_error"));
        assert!(output_string.contains("Error: Missing required fields: [], Missing required attributes: [self_program, fee_payer, authority]"));
    }

    #[test]
    fn test_process_fields_and_attributes_missing_fields() {
        let fields: FieldsNamed = parse_quote! {
            {
                #[self_program]
                pub my_program: Program<'info, MyProgram>,
                #[fee_payer]
                pub payer: Signer<'info>,
                pub user: AccountInfo<'info>, // missing #[authority]
                // Missing required fields
            }
        };

        let name = syn::Ident::new("TestStruct", proc_macro2::Span::call_site());
        let output = process_fields_and_attributes(&name, fields);
        let output_string = output.to_string();

        assert!(output_string.contains("compile_error"));
        assert!(output_string.contains("Error: Missing required fields: [light_system_program, registered_program_pda, noop_program, account_compression_authority, account_compression_program, system_program], Missing required attributes: [authority]"));
    }
}
