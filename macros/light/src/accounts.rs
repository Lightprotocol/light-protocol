use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::PathSep,
    Error, Expr, Fields, Ident, ItemStruct, Meta, Path, PathSegment, Result, Token, Type, TypePath,
};

pub(crate) fn process_light_system_accounts(input: ItemStruct) -> Result<TokenStream> {
    let mut output = input.clone();

    let fields =
        match output.fields {
            Fields::Named(ref mut fields) => fields,
            _ => return Err(Error::new_spanned(
                input,
                "`light_system_accounts` attribute can only be used with structs that have named fields.",
            )),
        };

    let fields_to_add = [
        (
            "light_system_program",
            "Program<'info, ::light_system_program::program::LightSystemProgram>",
        ),
        ("system_program", "Program<'info, System>"),
        (
            "account_compression_program",
            "Program<'info, ::account_compression::program::AccountCompression>",
        ),
    ];
    let fields_to_add_check = [
        (
            "registered_program_pda",
            "Account<'info, ::account_compression::RegisteredProgram>",
        ),
        ("noop_program", "AccountInfo<'info>"),
        ("account_compression_authority", "AccountInfo<'info>"),
    ];
    let existing_field_names: Vec<_> = fields
        .named
        .iter()
        .map(|f| f.ident.as_ref().unwrap().to_string())
        .collect();

    // TODO: Eventually we want to provide flexibility to override.
    // Until then, we error if the fields are manually defined.
    for (field_name, field_type) in fields_to_add.iter().chain(fields_to_add_check.iter()) {
        if existing_field_names.contains(&field_name.to_string()) {
            return Err(syn::Error::new_spanned(
                &output,
                format!("Field `{}` already exists in the struct.", field_name),
            ));
        }

        let new_field = syn::Field {
            attrs: vec![],
            vis: syn::Visibility::Public(syn::token::Pub {
                span: proc_macro2::Span::call_site(),
            }),
            mutability: syn::FieldMutability::None,
            ident: Some(syn::Ident::new(field_name, proc_macro2::Span::call_site())),
            colon_token: Some(syn::Token![:](proc_macro2::Span::call_site())),
            ty: syn::parse_str(field_type)?,
        };
        fields.named.push(new_field);
    }

    let expanded = quote! {
        #output
    };

    Ok(expanded)
}

pub(crate) fn process_light_accounts(input: ItemStruct) -> Result<TokenStream> {
    let mut anchor_accounts_strct = input.clone();

    let (_, type_gen, _) = input.generics.split_for_impl();

    let anchor_accounts_name = input.ident.clone();
    let light_accounts_name = Ident::new(&format!("Light{}", input.ident), Span::call_site());

    let mut light_accounts_fields: Punctuated<syn::Field, Token![,]> = Punctuated::new();

    let fields =
        match anchor_accounts_strct.fields {
            Fields::Named(ref mut fields) => fields,
            _ => return Err(Error::new_spanned(
                input,
                "`light_accounts` attribute can only be used with structs that have named fields.",
            )),
        };

    let mut anchor_fields = Punctuated::new();
    let mut anchor_field_idents = Vec::new();
    let mut light_field_idents = Vec::new();
    let mut derive_address_seed_calls = Vec::new();

    for field in fields.named.iter() {
        let mut light_account = false;
        for attr in &field.attrs {
            if attr.path().is_ident("light_account") {
                light_account = true;
            }
        }

        if light_account {
            light_accounts_fields.push(field.clone());
            light_field_idents.push(field.ident.clone());

            let field_ident = &field.ident;

            let mut account_args = None;
            for attribute in &field.attrs {
                let attribute_list = match &attribute.meta {
                    Meta::List(attribute_list) => attribute_list,
                    _ => continue,
                };
                account_args = Some(syn::parse2::<LightAccountArgs>(
                    attribute_list.tokens.clone(),
                )?);
                break;
            }
            let account_args = match account_args {
                Some(account_args) => account_args,
                None => {
                    return Err(Error::new_spanned(
                        input,
                        "no arguments provided in `light_account`",
                    ))
                }
            };

            let seeds = account_args.seeds;

            derive_address_seed_calls.push(quote! {
                let address_seed = ::light_sdk::address::derive_address_seed(
                    &#seeds,
                    &crate::ID,
                    &unpacked_address_merkle_context,
                );
                #field_ident.set_address_seed(address_seed);
            });
        } else {
            anchor_fields.push(field.clone());
            anchor_field_idents.push(field.ident.clone());
        }
    }

    fields.named = anchor_fields;

    let light_accounts_strct = if light_accounts_fields.is_empty() {
        quote! {
            #[derive(::light_sdk::LightAccounts)]
            pub struct #light_accounts_name {}
        }
    } else {
        quote! {
            #[derive(::light_sdk::LightAccounts)]
            pub struct #light_accounts_name {
                #light_accounts_fields
            }
        }
    };

    let expanded = quote! {
        #[::light_sdk::light_system_accounts]
        #[derive(::anchor_lang::Accounts, ::light_sdk::LightTraits)]
        #anchor_accounts_strct

        #light_accounts_strct

        impl<'a, 'b, 'c, 'info> LightContextExt for ::light_sdk::context::LightContext<
            'a, 'b, 'c, 'info, #anchor_accounts_name #type_gen, #light_accounts_name,
        > {
            #[allow(unused_variables)]
            fn derive_address_seeds(
                &mut self,
                address_merkle_context: ::light_sdk::merkle_context::PackedAddressMerkleContext,
            ) {
                let #anchor_accounts_name { #(#anchor_field_idents),*, .. } = &self.anchor_context.accounts;
                let #light_accounts_name { #(#light_field_idents),* } = &mut self.light_accounts;

                let unpacked_address_merkle_context =
                    ::light_sdk::program_merkle_context::unpack_address_merkle_context(
                        address_merkle_context, self.anchor_context.remaining_accounts);

                #(#derive_address_seed_calls)*
            }
        }
    };

    Ok(expanded)
}

pub(crate) enum LightAccountAction {
    Init,
    Mut,
    Close,
}

pub(crate) struct LightAccountArgs {
    action: LightAccountAction,
    seeds: Expr,
}

impl Parse for LightAccountArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let action = match input.parse::<Token![mut]>() {
            Ok(_) => LightAccountAction::Mut,
            Err(_) => {
                let action_ident: Ident = input.parse()?;
                match action_ident.to_string().as_str() {
                    "init" => LightAccountAction::Init,
                    "mut" => LightAccountAction::Mut,
                    "close" => LightAccountAction::Close,
                    _ => {
                        return Err(Error::new(
                            Span::call_site(),
                            "unsupported light account action type",
                        ))
                    }
                }
            }
        };

        input.parse::<Token![,]>()?;

        let _seeds_ident: Ident = input.parse()?;

        input.parse::<Token![=]>()?;

        let seeds: Expr = input.parse()?;

        Ok(Self { action, seeds })
    }
}

pub(crate) fn process_light_accounts_derive(input: ItemStruct) -> Result<TokenStream> {
    let strct_name = &input.ident;
    let (impl_gen, type_gen, where_clause) = input.generics.split_for_impl();

    let mut try_from_slice_calls = Vec::new();
    let mut field_idents = Vec::new();
    let mut new_address_params_calls = Vec::new();
    let mut input_account_calls = Vec::new();
    let mut output_account_calls = Vec::new();

    let fields = match input.fields {
        Fields::Named(ref fields) => fields,
        _ => {
            return Err(Error::new_spanned(
                input,
                "Only structs with named fields can derive LightAccounts",
            ))
        }
    };

    for (i, field) in fields.named.iter().enumerate() {
        let field_ident = &field.ident;
        field_idents.push(field_ident);

        let mut account_args = None;
        for attribute in &field.attrs {
            let attribute_list = match &attribute.meta {
                Meta::List(attribute_list) => attribute_list,
                _ => continue,
            };
            account_args = Some(syn::parse2::<LightAccountArgs>(
                attribute_list.tokens.clone(),
            )?);
            break;
        }
        let account_args = match account_args {
            Some(account_args) => account_args,
            None => {
                return Err(Error::new_spanned(
                    input,
                    "no arguments provided in `light_account`",
                ))
            }
        };

        let type_path = match field.ty {
            Type::Path(ref type_path) => type_path,
            _ => {
                return Err(Error::new_spanned(
                    input,
                    "Only struct with typed fields can derive LightAccounts",
                ))
            }
        };

        let type_path_without_args = TypePath {
            qself: type_path.qself.clone(),
            path: Path {
                leading_colon: type_path.path.leading_colon,
                segments: type_path
                    .path
                    .segments
                    .iter()
                    .map(|segment| PathSegment {
                        ident: segment.ident.clone(),
                        arguments: syn::PathArguments::None,
                    })
                    .collect::<Punctuated<PathSegment, PathSep>>(),
            },
        };
        let try_from_slice_call = match account_args.action {
            LightAccountAction::Init => quote! {
                let mut #field_ident: #type_path = #type_path_without_args::new_init(
                    &output_merkle_context,
                    &address_merkle_context,
                    address_merkle_tree_root_index,
                );
            },
            LightAccountAction::Mut => quote! {
                let mut #field_ident: #type_path = #type_path_without_args::try_from_slice_mut(
                    inputs[#i].as_slice(),
                    &input_merkle_context,
                    input_merkle_tree_root_index,
                    &output_merkle_context,
                    &address_merkle_context,
                )?;
            },
            LightAccountAction::Close => quote! {
                let mut #field_ident: #type_path = #type_path_without_args::try_from_slice_close(
                    inputs[#i].as_slice(),
                    &input_merkle_context,
                    input_merkle_tree_root_index,
                    &address_merkle_context,
                )?;
            },
        };
        try_from_slice_calls.push(try_from_slice_call);

        new_address_params_calls.push(quote! {
            if let Some(new_address_params_for_acc) = self.#field_ident.new_address_params() {
                new_address_params.push(new_address_params_for_acc);
            }
        });
        input_account_calls.push(quote! {
            if let Some(compressed_account) = self.#field_ident.input_compressed_account(
                &crate::ID,
                remaining_accounts,
            )? {
                accounts.push(compressed_account);
            }
        });
        output_account_calls.push(quote! {
            if let Some(compressed_account) = self.#field_ident.output_compressed_account(
                &crate::ID,
                remaining_accounts,
            )? {
                accounts.push(compressed_account);
            }
        })
    }

    let expanded = quote! {
        impl #impl_gen ::light_sdk::compressed_account::LightAccounts for #strct_name #type_gen #where_clause {
            fn try_light_accounts(
                inputs: Vec<Vec<u8>>,
                input_merkle_context: ::light_sdk::merkle_context::PackedMerkleContext,
                input_merkle_tree_root_index: u16,
                output_merkle_context: ::light_sdk::merkle_context::PackedMerkleOutputContext,
                address_merkle_context: ::light_sdk::merkle_context::PackedAddressMerkleContext,
                address_merkle_tree_root_index: u16,
                remaining_accounts: &[::anchor_lang::prelude::AccountInfo],
            ) -> Result<Self> {
                let unpacked_address_merkle_context =
                     ::light_sdk::program_merkle_context::unpack_address_merkle_context(
                         address_merkle_context, remaining_accounts);

                #(#try_from_slice_calls)*
                Ok(Self {
                    #(#field_idents),*
                })
            }

            fn new_address_params(&self) -> Vec<::light_sdk::compressed_account::NewAddressParamsPacked> {
                let mut new_address_params = Vec::new();
                #(#new_address_params_calls)*
                new_address_params
            }

            fn input_accounts(&self, remaining_accounts: &[::anchor_lang::prelude::AccountInfo]) -> Result<Vec<::light_sdk::compressed_account::PackedCompressedAccountWithMerkleContext>> {
                let mut accounts = Vec::new();
                #(#input_account_calls)*
                Ok(accounts)
            }

            fn output_accounts(&self, remaining_accounts: &[::anchor_lang::prelude::AccountInfo]) -> Result<Vec<::light_sdk::compressed_account::OutputCompressedAccountWithPackedContext>> {
                let mut accounts = Vec::new();
                #(#output_account_calls)*
                Ok(accounts)
            }
        }
    };

    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, ItemStruct};

    #[test]
    fn test_process_light_system_accounts_adds_fields_correctly() {
        let input: ItemStruct = parse_quote! {
            struct TestStruct {
                #[light_account(mut)]
                foo: u64,
                existing_field: u32,
            }
        };

        let output = process_light_system_accounts(input).unwrap();
        let output_string = output.to_string();

        println!("{output_string}");

        assert!(output_string.contains("light_system_program"));
        assert!(output_string.contains("system_program"));
        assert!(output_string.contains("account_compression_program"));
        assert!(output_string.contains("registered_program_pda"));
        assert!(output_string.contains("noop_program"));
        assert!(output_string.contains("account_compression_authority"));
    }

    #[test]
    fn test_process_light_system_accounts_fails_on_existing_field() {
        let input: ItemStruct = parse_quote! {
            struct TestStruct {
                existing_field: u32,
                system_program: Program<'info, System>,
            }
        };

        let result = process_light_system_accounts(input);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Field `system_program` already exists in the struct."));
    }
}
