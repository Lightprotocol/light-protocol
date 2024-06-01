extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

// use light_traits::InvokeCpiAccounts;

#[proc_macro_derive(AutoTraits, attributes(invoke_cpi))]
pub fn auto_traits_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let trait_impls = match input.data {
        Data::Struct(data_struct) => {
            match data_struct.fields {
                Fields::Named(fields) => {
                    fields.named.iter().filter_map(|f| {
                        if f.attrs.iter().any(|attr| attr.path.is_ident("invoke_cpi")) {
                            let field_name = &f.ident;
                            Some(quote! {
                                impl<'info> InvokeCpiAccounts<'info> for #name<'info> {
                                    fn get_invoking_program(&self) -> &AccountInfo<'info> {
                                        &self.#field_name
                                    }
                                }
                            })
                        } else {
                            None
                        }
                    }).collect()
                },
                _ => quote! {
                    panic!("Error: Expected named fields but found unnamed or no fields.");
                },
            }
        },
        _ => quote! {},
    };

    let expanded = quote! {
        #trait_impls
    };

    TokenStream::from(expanded)
}



// #[proc_macro_derive(AutoTraits2, attributes(invoke_cpi, fee_payer, authority))]
// pub fn auto_traits_derive(input: TokenStream) -> TokenStream {
//     let input = parse_macro_input!(input as DeriveInput);
//     let name = &input.ident;

//     let mut invoke_accounts_impl = quote! {};
//     let mut invoke_cpi_accounts_impl = quote! {};
//     let mut signer_accounts_impl = quote! {};
//     let mut light_system_account_impl = quote! {};
//     let mut cpi_context_account_impl = quote! {};

//     let mut has_compressed_sol_pda = false;
//     let mut has_compression_recipient = false;
//     let mut has_light_system_program = false;
//     let mut has_cpi_context_account = false;
//     // required
//     let mut has_registered_program_pda = false;
//     let mut has_noop_program = false;
//     let mut has_account_compression_authority = false;
//     let mut has_account_compression_program = false;
//     let mut has_system_program = false;

//     if let Data::Struct(data_struct) = &input.data {
//         if let Fields::Named(fields) = &data_struct.fields {
//             for f in &fields.named {
//                 let field_name = &f.ident;
//                 let field_str = field_name.as_ref().unwrap().to_string();

//                 // Check for special fields
//                 match field_str.as_str() {
//                     "compressed_sol_pda" => has_compressed_sol_pda = true,
//                     "compression_recipient" => has_compression_recipient = true,
//                     "light_system_program" => has_light_system_program = true,
//                     "cpi_context_account" => has_cpi_context_account = true,
//                     "registered_program_pda" => has_registered_program_pda = true,
//                     "noop_program" => has_noop_program = true,
//                     "account_compression_authority" => has_account_compression_authority = true,
//                     "account_compression_program" => has_account_compression_program = true,
//                     "system_program" => has_system_program = true,
//                     _ => {}
//                 }

//                 // Generate implementations based on attributes
//                 for attr in &f.attrs {
//                     if attr.path.is_ident("invoke_cpi") {
//                         invoke_cpi_accounts_impl = quote! {
//                             impl<'info> InvokeCpiAccounts<'info> for #name<'info> {
//                                 fn get_invoking_program(&self) -> &AccountInfo<'info> {
//                                     &self.#field_name
//                                 }
//                             }
//                         };
//                     } else if attr.path.is_ident("fee_payer") {
//                         signer_accounts_impl = quote! {
//                             impl<'info> SignerAccounts<'info> for #name<'info> {
//                                 fn get_fee_payer(&self) -> &Signer<'info> {
//                                     &self.#field_name
//                                 }
//                             }
//                         };
//                     } else if attr.path.is_ident("authority") {
//                         signer_accounts_impl.extend(quote! {
//                             impl<'info> SignerAccounts<'info> for #name<'info> {
//                                 fn get_authority(&self) -> &AccountInfo<'info> {
//                                     &self.#field_name
//                                 }
//                             }
//                         });
//                     }
//                 }
//             }
//         }
//     }

//     // Implement InvokeAccounts with optional fields
//     invoke_accounts_impl = quote! {
//         impl<'info> InvokeAccounts<'info> for #name<'info> {
//             fn get_compressed_sol_pda(&self) -> Option<&UncheckedAccount<'info>> {
//                 if #has_compressed_sol_pda {
//                     Some(&self.compressed_sol_pda)
//                 } else {
//                     None
//                 }
//             }

//             fn get_compression_recipient(&self) -> Option<&UncheckedAccount<'info>> {
//                 if #has_compression_recipient {
//                     Some(&self.compression_recipient)
//                 } else {
//                     None
//                 }
//             }

//             if #has_registered_program_pda {
//                 fn get_registered_program_pda(&self) -> &Account<'info, account_compression::instructions::register_program::RegisteredProgram> {
//                     &self.registered_program_pda
//                 }
//             } else {
//                 panic!("registered_program_pda field is required but not provided!");
//             }

//             if #has_noop_program {
//                 fn get_noop_program(&self) -> &AccountInfo<'info> {
//                     &self.noop_program
//                 }
//             } else {
//                 panic!("noop_program field is required but not provided!");
//             }

//             if #has_account_compression_authority {
//                 fn get_account_compression_authority(&self) -> &AccountInfo<'info> {
//                     &self.account_compression_authority
//                 }
//             } else {
//                 panic!("account_compression_authority field is required but not provided!");
//             }


//             if #has_account_compression_program {
//                 fn get_account_compression_program(&self) -> &Program<'info, AccountCompression> {
//                     &self.account_compression_program
//                 }
//             } else {
//                 panic!("account_compression_program field is required but not provided!");
//             }

//             if #has_system_program {
//                 fn get_system_program(&self) -> &Program<'info, System> {
//                     &self.system_program
//                 }
//             } else {
//                 panic!("system_program field is required but not provided!");
//             }
//         }
//     };
    
//     // Required: LightSystemProgram
//     light_system_account_impl = if has_light_system_program {
//         quote! {
//             impl<'info> LightSystemAccount<'info> for #name<'info> {
//                 fn get_light_system_program(&self) -> &Program<'info, LightSystemProgram> {
//                     &self.light_system_program
//                 }
//             }
//         }
//     } else {
//         panic!("light_system_program field is required but not provided!");
//     };


//     // Optional: CpiContextAccount
//     cpi_context_account_impl = quote! {
//         impl<'info> InvokeCpiContextAccount<'info> for #name<'info> {
//             fn get_cpi_context_account(&self) -> Option<&Account<'info, CpiContextAccount>> {
//                 if #has_cpi_context_account {
//                     Some(&self.cpi_context_account)
//                 } else {
//                     None
//                 }
//             }
//         }
//     };

//     let expanded = quote! {
//         #invoke_accounts_impl
//         #invoke_cpi_accounts_impl
//         #signer_accounts_impl
//         #light_system_account_impl
//         #cpi_context_account_impl
//     };

//     TokenStream::from(expanded)
// }