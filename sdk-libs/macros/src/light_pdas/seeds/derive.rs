use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Fields, Type};

use super::extract::{extract_seeds_from_attribute, parse_instruction_args};
use super::types::SeedKind;
use std::collections::HashSet;

pub fn derive_seed(input: DeriveInput) -> syn::Result<TokenStream> {
    let struct_name = &input.ident;

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            _ => return Ok(quote! {}),
        },
        _ => return Ok(quote! {}),
    };

    let instruction_args = parse_instruction_args(&input.attrs)?;
    let params_type = parse_instruction_params_type(&input.attrs)?;

    let account_fields: HashSet<String> = fields
        .iter()
        .filter_map(|f| f.ident.as_ref().map(|i| i.to_string()))
        .collect();

    let mut seed_impls = Vec::new();

    for field in fields {
        let seeds = extract_seeds_from_attribute(&field.attrs, &instruction_args, &account_fields)?;

        if seeds.is_empty() {
            continue;
        }

        let mut seed_exprs = Vec::new();
        for seed in &seeds {
            let expr = match seed.kind {
                SeedKind::Constant => {
                    let e = &seed.expr;
                    quote! { #e }
                }
                SeedKind::Account => {
                    let f = seed.field.as_ref().unwrap();
                    quote! { accounts.#f.key.as_ref() }
                }
                SeedKind::Data => {
                    let f = seed.field.as_ref().unwrap();
                    quote! { params.#f.as_ref() }
                }
            };
            seed_exprs.push(expr);
        }

        seed_exprs.push(quote! { &params.bump });

        let seed_count = seed_exprs.len();

        if let Some(ref params_ty) = params_type {
            seed_impls.push(quote! {
                impl<'info> ::light_sdk::PdaSeeds<#struct_name<'info>, #seed_count> for #params_ty {
                    fn seeds<'a>(&'a self, accounts: &'a #struct_name<'info>) -> [&'a [u8]; #seed_count] {
                        let params = self;
                        [#(#seed_exprs),*]
                    }
                }
            });
        }
    }

    Ok(quote! { #(#seed_impls)* })
}

fn parse_instruction_params_type(attrs: &[syn::Attribute]) -> syn::Result<Option<Type>> {
    for attr in attrs {
        if attr.path().is_ident("instruction") {
            let ty = attr.parse_args_with(|input: syn::parse::ParseStream| {
                let _name: syn::Ident = input.parse()?;
                input.parse::<syn::Token![:]>()?;
                let ty: Type = input.parse()?;
                Ok(ty)
            })?;
            return Ok(Some(ty));
        }
    }
    Ok(None)
}
