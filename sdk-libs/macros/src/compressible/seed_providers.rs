//! Seed provider generation for PDA and CToken accounts.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, Ident, Result};

use crate::compressible::instructions::{InstructionDataSpec, SeedElement, TokenSeedSpec};

pub fn generate_ctoken_account_variant_enum(token_seeds: &[TokenSeedSpec]) -> Result<TokenStream> {
    let variants = token_seeds.iter().enumerate().map(|(index, spec)| {
        let variant_name = &spec.variant;
        let index_u8 = index as u8;
        quote! {
            #variant_name = #index_u8,
        }
    });

    Ok(quote! {
        #[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
        #[repr(u8)]
        pub enum CTokenAccountVariant {
            #(#variants)*
        }
    })
}

pub fn generate_ctoken_seed_provider_implementation(
    token_seeds: &[TokenSeedSpec],
) -> Result<TokenStream> {
    let mut get_seeds_match_arms = Vec::new();
    let mut get_authority_seeds_match_arms = Vec::new();

    for spec in token_seeds {
        let variant_name = &spec.variant;

        if spec.is_ata {
            let get_seeds_arm = quote! {
                CTokenAccountVariant::#variant_name => {
                    Err(anchor_lang::prelude::ProgramError::Custom(
                        CompressibleInstructionError::AtaDoesNotUseSeedDerivation.into()
                    ).into())
                }
            };
            get_seeds_match_arms.push(get_seeds_arm);

            let authority_arm = quote! {
                CTokenAccountVariant::#variant_name => {
                    Err(anchor_lang::prelude::ProgramError::Custom(
                        CompressibleInstructionError::AtaDoesNotUseSeedDerivation.into()
                    ).into())
                }
            };
            get_authority_seeds_match_arms.push(authority_arm);
            continue;
        }

        let mut token_bindings = Vec::new();
        let mut token_seed_refs = Vec::new();

        for (i, seed) in spec.seeds.iter().enumerate() {
            match seed {
                SeedElement::Literal(lit) => {
                    let value = lit.value();
                    token_seed_refs.push(quote! { #value.as_bytes() });
                }
                SeedElement::Expression(expr) => {
                    if let syn::Expr::Path(path_expr) = &**expr {
                        if let Some(ident) = path_expr.path.get_ident() {
                            let ident_str = ident.to_string();
                            if ident_str
                                .chars()
                                .all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit())
                            {
                                if ident_str == "LIGHT_CPI_SIGNER" {
                                    token_seed_refs.push(quote! { #ident.cpi_signer.as_ref() });
                                } else {
                                    token_seed_refs.push(quote! { #ident.as_bytes() });
                                }
                                continue;
                            }
                        }
                    }

                    let mut handled = false;
                    if let syn::Expr::Field(field_expr) = &**expr {
                        if let syn::Member::Named(field_name) = &field_expr.member {
                            if let syn::Expr::Field(nested_field) = &*field_expr.base {
                                if let syn::Member::Named(base_name) = &nested_field.member {
                                    if base_name == "accounts" {
                                        if let syn::Expr::Path(path) = &*nested_field.base {
                                            if let Some(segment) = path.path.segments.first() {
                                                if segment.ident == "ctx" {
                                                    let binding_name = syn::Ident::new(
                                                        &format!("seed_{}", i),
                                                        expr.span(),
                                                    );
                                                    let field_name_str = field_name.to_string();
                                                    let is_standard_field = matches!(
                                                        field_name_str.as_str(),
                                                        "fee_payer"
                                                            | "rent_sponsor"
                                                            | "config"
                                                            | "compression_authority"
                                                    );
                                                    if is_standard_field {
                                                        token_bindings.push(quote! {
                                                            let #binding_name = ctx.accounts.#field_name.key();
                                                        });
                                                    } else {
                                                        token_bindings.push(quote! {
                                                            let #binding_name = ctx.accounts.#field_name
                                                                .as_ref()
                                                                .ok_or_else(|| -> anchor_lang::error::Error {
                                                                    anchor_lang::prelude::ProgramError::Custom(
                                                                        CompressibleInstructionError::MissingSeedAccount.into()
                                                                    ).into()
                                                                })?
                                                                .key();
                                                        });
                                                    }
                                                    token_seed_refs
                                                        .push(quote! { #binding_name.as_ref() });
                                                    handled = true;
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if let syn::Expr::Path(path) = &*field_expr.base {
                                if let Some(segment) = path.path.segments.first() {
                                    if segment.ident == "ctx" {
                                        let binding_name =
                                            syn::Ident::new(&format!("seed_{}", i), expr.span());
                                        let field_name_str = field_name.to_string();
                                        let is_standard_field = matches!(
                                            field_name_str.as_str(),
                                            "fee_payer"
                                                | "rent_sponsor"
                                                | "config"
                                                | "compression_authority"
                                        );
                                        if is_standard_field {
                                            token_bindings.push(quote! {
                                                let #binding_name = ctx.accounts.#field_name.key();
                                            });
                                        } else {
                                            token_bindings.push(quote! {
                                                let #binding_name = ctx.accounts.#field_name
                                                    .as_ref()
                                                    .ok_or_else(|| -> anchor_lang::error::Error {
                                                        anchor_lang::prelude::ProgramError::Custom(
                                                            CompressibleInstructionError::MissingSeedAccount.into()
                                                        ).into()
                                                    })?
                                                    .key();
                                            });
                                        }
                                        token_seed_refs.push(quote! { #binding_name.as_ref() });
                                        handled = true;
                                    }
                                }
                            }
                        }
                    }

                    if !handled {
                        token_seed_refs.push(quote! { (#expr).as_ref() });
                    }
                }
            }
        }

        let get_seeds_arm = quote! {
            CTokenAccountVariant::#variant_name => {
                #(#token_bindings)*
                let seeds: &[&[u8]] = &[#(#token_seed_refs),*];
                let (token_account_pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, &crate::ID);
                let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                seeds_vec.extend(seeds.iter().map(|s| s.to_vec()));
                seeds_vec.push(vec![bump]);
                Ok((seeds_vec, token_account_pda))
            }
        };
        get_seeds_match_arms.push(get_seeds_arm);

        if let Some(authority_seeds) = &spec.authority {
            let mut auth_bindings: Vec<TokenStream> = Vec::new();
            let mut auth_seed_refs = Vec::new();

            for (i, authority_seed) in authority_seeds.iter().enumerate() {
                match authority_seed {
                    SeedElement::Literal(lit) => {
                        let value = lit.value();
                        auth_seed_refs.push(quote! { #value.as_bytes() });
                    }
                    SeedElement::Expression(expr) => {
                        let mut handled = false;
                        match &**expr {
                            syn::Expr::Field(field_expr) => {
                                if let syn::Member::Named(field_name) = &field_expr.member {
                                    if let syn::Expr::Field(nested_field) = &*field_expr.base {
                                        if let syn::Member::Named(base_name) = &nested_field.member
                                        {
                                            if base_name == "accounts" {
                                                if let syn::Expr::Path(path) = &*nested_field.base {
                                                    if let Some(segment) =
                                                        path.path.segments.first()
                                                    {
                                                        if segment.ident == "ctx" {
                                                            let binding_name = syn::Ident::new(
                                                                &format!("authority_seed_{}", i),
                                                                expr.span(),
                                                            );
                                                            let field_name_str =
                                                                field_name.to_string();
                                                            let is_standard_field = matches!(
                                                                field_name_str.as_str(),
                                                                "fee_payer"
                                                                    | "rent_sponsor"
                                                                    | "config"
                                                                    | "compression_authority"
                                                            );
                                                            if is_standard_field {
                                                                auth_bindings.push(quote! {
                                                                    let #binding_name = ctx.accounts.#field_name.key();
                                                                });
                                                            } else {
                                                                auth_bindings.push(quote! {
                                                                    let #binding_name = ctx.accounts.#field_name
                                                                        .as_ref()
                                                                        .ok_or_else(|| -> anchor_lang::error::Error {
                                                                            anchor_lang::prelude::ProgramError::Custom(
                                                                                CompressibleInstructionError::MissingSeedAccount.into()
                                                                            ).into()
                                                                        })?
                                                                        .key();
                                                                });
                                                            }
                                                            auth_seed_refs.push(
                                                                quote! { #binding_name.as_ref() },
                                                            );
                                                            handled = true;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if let syn::Expr::Path(path) = &*field_expr.base {
                                        if let Some(segment) = path.path.segments.first() {
                                            if segment.ident == "ctx" {
                                                let binding_name = syn::Ident::new(
                                                    &format!("authority_seed_{}", i),
                                                    expr.span(),
                                                );
                                                let field_name_str = field_name.to_string();
                                                let is_standard_field = matches!(
                                                    field_name_str.as_str(),
                                                    "fee_payer"
                                                        | "rent_sponsor"
                                                        | "config"
                                                        | "compression_authority"
                                                );
                                                if is_standard_field {
                                                    auth_bindings.push(quote! {
                                                        let #binding_name = ctx.accounts.#field_name.key();
                                                    });
                                                } else {
                                                    auth_bindings.push(quote! {
                                                        let #binding_name = ctx.accounts.#field_name
                                                            .as_ref()
                                                            .ok_or_else(|| -> anchor_lang::error::Error {
                                                                anchor_lang::prelude::ProgramError::Custom(
                                                                    CompressibleInstructionError::MissingSeedAccount.into()
                                                                ).into()
                                                            })?
                                                            .key();
                                                    });
                                                }
                                                auth_seed_refs
                                                    .push(quote! { #binding_name.as_ref() });
                                                handled = true;
                                            }
                                        }
                                    }
                                }
                            }
                            syn::Expr::MethodCall(_mc) => {
                                auth_seed_refs.push(quote! { (#expr).as_ref() });
                                handled = true;
                            }
                            syn::Expr::Path(path_expr) => {
                                if let Some(ident) = path_expr.path.get_ident() {
                                    let ident_str = ident.to_string();
                                    if ident_str.chars().all(|c| c.is_uppercase() || c == '_') {
                                        if ident_str == "LIGHT_CPI_SIGNER" {
                                            auth_seed_refs
                                                .push(quote! { #ident.cpi_signer.as_ref() });
                                        } else {
                                            auth_seed_refs.push(quote! { #ident.as_bytes() });
                                        }
                                        handled = true;
                                    }
                                }
                            }
                            _ => {}
                        }

                        if !handled {
                            auth_seed_refs.push(quote! { (#expr).as_ref() });
                        }
                    }
                }
            }

            let authority_arm = quote! {
                CTokenAccountVariant::#variant_name => {
                    #(#auth_bindings)*
                    let seeds: &[&[u8]] = &[#(#auth_seed_refs),*];
                    let (authority_pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, &crate::ID);
                    let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                    seeds_vec.extend(seeds.iter().map(|s| s.to_vec()));
                    seeds_vec.push(vec![bump]);
                    Ok((seeds_vec, authority_pda))
                }
            };
            get_authority_seeds_match_arms.push(authority_arm);
        } else {
            let authority_arm = quote! {
                CTokenAccountVariant::#variant_name => {
                    Err(anchor_lang::prelude::ProgramError::Custom(
                        CompressibleInstructionError::MissingSeedAccount.into()
                    ).into())
                }
            };
            get_authority_seeds_match_arms.push(authority_arm);
        }
    }

    Ok(quote! {
        impl ctoken_seed_system::CTokenSeedProvider for CTokenAccountVariant {
            fn get_seeds<'a, 'info>(
                &self,
                ctx: &ctoken_seed_system::CTokenSeedContext<'a, 'info>,
            ) -> Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey)> {
                match self {
                    #(#get_seeds_match_arms)*
                    _ => Err(anchor_lang::prelude::ProgramError::Custom(
                        CompressibleInstructionError::MissingSeedAccount.into()
                    ).into())
                }
            }

            fn get_authority_seeds<'a, 'info>(
                &self,
                ctx: &ctoken_seed_system::CTokenSeedContext<'a, 'info>,
            ) -> Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey)> {
                match self {
                    #(#get_authority_seeds_match_arms)*
                    _ => Err(anchor_lang::prelude::ProgramError::Custom(
                        CompressibleInstructionError::MissingSeedAccount.into()
                    ).into())
                }
            }
        }
    })
}

#[inline(never)]
pub fn generate_client_seed_functions(
    _account_types: &[Ident],
    pda_seeds: &Option<Vec<TokenSeedSpec>>,
    token_seeds: &Option<Vec<TokenSeedSpec>>,
    instruction_data: &[InstructionDataSpec],
) -> Result<TokenStream> {
    let mut functions = Vec::new();

    if let Some(pda_seed_specs) = pda_seeds {
        for spec in pda_seed_specs {
            let variant_name = &spec.variant;
            let snake_case = camel_to_snake_case(&variant_name.to_string());
            let function_name = format_ident!("get_{}_seeds", snake_case);

            let (parameters, seed_expressions) =
                analyze_seed_spec_for_client(spec, instruction_data)?;

            let seed_count = seed_expressions.len();
            let function = quote! {
                pub fn #function_name(#(#parameters),*) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
                    let mut seed_values = Vec::with_capacity(#seed_count + 1);
                    #(
                        seed_values.push((#seed_expressions).to_vec());
                    )*
                let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
                    let (pda, bump) = solana_pubkey::Pubkey::find_program_address(&seed_slices, &crate::ID);
                    seed_values.push(vec![bump]);
                    (seed_values, pda)
                }
            };
            functions.push(function);
        }
    }

    if let Some(token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            let variant_name = &spec.variant;

            if spec.is_ata {
                continue;
            }

            let function_name =
                format_ident!("get_{}_seeds", variant_name.to_string().to_lowercase());

            let (parameters, seed_expressions) =
                analyze_seed_spec_for_client(spec, instruction_data)?;

            let seed_count = seed_expressions.len();
            let function = quote! {
                pub fn #function_name(#(#parameters),*) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
                    let mut seed_values = Vec::with_capacity(#seed_count + 1);
                    #(
                        seed_values.push((#seed_expressions).to_vec());
                    )*
                    let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
                    let (pda, bump) = solana_pubkey::Pubkey::find_program_address(&seed_slices, &crate::ID);
                    seed_values.push(vec![bump]);
                    (seed_values, pda)
                }
            };
            functions.push(function);

            if let Some(authority_seeds) = &spec.authority {
                let authority_function_name = format_ident!(
                    "get_{}_authority_seeds",
                    variant_name.to_string().to_lowercase()
                );

                let mut authority_spec = TokenSeedSpec {
                    variant: spec.variant.clone(),
                    _eq: spec._eq,
                    is_token: spec.is_token,
                    is_ata: spec.is_ata,
                    seeds: syn::punctuated::Punctuated::new(),
                    authority: None,
                };

                for auth_seed in authority_seeds {
                    authority_spec.seeds.push(auth_seed.clone());
                }

                let (auth_parameters, auth_seed_expressions) =
                    analyze_seed_spec_for_client(&authority_spec, instruction_data)?;

                let auth_seed_count = auth_seed_expressions.len();
                let (fn_params, fn_body) = if auth_parameters.is_empty() {
                    (
                        quote! { _program_id: &solana_pubkey::Pubkey },
                        quote! {
                            let mut seed_values = Vec::with_capacity(#auth_seed_count + 1);
                            #(
                                seed_values.push((#auth_seed_expressions).to_vec());
                            )*
                            let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
                            let (pda, bump) = solana_pubkey::Pubkey::find_program_address(&seed_slices, _program_id);
                            seed_values.push(vec![bump]);
                            (seed_values, pda)
                        },
                    )
                } else {
                    (
                        quote! { #(#auth_parameters),* },
                        quote! {
                            let mut seed_values = Vec::with_capacity(#auth_seed_count + 1);
                            #(
                                seed_values.push((#auth_seed_expressions).to_vec());
                            )*
                            let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
                            let (pda, bump) = solana_pubkey::Pubkey::find_program_address(&seed_slices, &crate::ID);
                            seed_values.push(vec![bump]);
                            (seed_values, pda)
                        },
                    )
                };
                let authority_function = quote! {
                    pub fn #authority_function_name(#fn_params) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
                        #fn_body
                    }
                };
                functions.push(authority_function);
            }
        }
    }

    Ok(quote! {
        mod __client_seed_functions {
            use super::*;
            #(#functions)*
        }

        pub use __client_seed_functions::*;
    })
}

#[inline(never)]
fn analyze_seed_spec_for_client(
    spec: &TokenSeedSpec,
    instruction_data: &[InstructionDataSpec],
) -> Result<(Vec<TokenStream>, Vec<TokenStream>)> {
    let mut parameters = Vec::new();
    let mut expressions = Vec::new();

    for seed in &spec.seeds {
        match seed {
            SeedElement::Literal(lit) => {
                let value = lit.value();
                expressions.push(quote! { #value.as_bytes() });
            }
            SeedElement::Expression(expr) => {
                match &**expr {
                    syn::Expr::Field(field_expr) => {
                        if let syn::Member::Named(field_name) = &field_expr.member {
                            match &*field_expr.base {
                                syn::Expr::Field(nested_field) => {
                                    if let syn::Member::Named(base_name) = &nested_field.member {
                                        if base_name == "accounts" {
                                            if let syn::Expr::Path(path) = &*nested_field.base {
                                                if let Some(_segment) = path.path.segments.first() {
                                                    parameters.push(quote! { #field_name: &solana_pubkey::Pubkey });
                                                    expressions
                                                        .push(quote! { #field_name.as_ref() });
                                                } else {
                                                    parameters.push(quote! { #field_name: &solana_pubkey::Pubkey });
                                                    expressions
                                                        .push(quote! { #field_name.as_ref() });
                                                }
                                            } else {
                                                parameters.push(
                                                    quote! { #field_name: &solana_pubkey::Pubkey },
                                                );
                                                expressions.push(quote! { #field_name.as_ref() });
                                            }
                                        } else {
                                            parameters.push(
                                                quote! { #field_name: &solana_pubkey::Pubkey },
                                            );
                                            expressions.push(quote! { #field_name.as_ref() });
                                        }
                                    } else {
                                        parameters
                                            .push(quote! { #field_name: &solana_pubkey::Pubkey });
                                        expressions.push(quote! { #field_name.as_ref() });
                                    }
                                }
                                syn::Expr::Path(path) => {
                                    if let Some(segment) = path.path.segments.first() {
                                        if segment.ident == "data" {
                                            if let Some(data_spec) = instruction_data
                                                .iter()
                                                .find(|d| d.field_name == *field_name)
                                            {
                                                let param_type = &data_spec.field_type;
                                                let param_with_ref = if is_pubkey_type(param_type) {
                                                    quote! { #field_name: &#param_type }
                                                } else {
                                                    quote! { #field_name: #param_type }
                                                };
                                                parameters.push(param_with_ref);
                                                expressions.push(quote! { #field_name.as_ref() });
                                            } else {
                                                return Err(syn::Error::new_spanned(
                                                    field_name,
                                                    format!("data.{} used in seeds but no type specified", field_name),
                                                ));
                                            }
                                        } else {
                                            parameters.push(
                                                quote! { #field_name: &solana_pubkey::Pubkey },
                                            );
                                            expressions.push(quote! { #field_name.as_ref() });
                                        }
                                    } else {
                                        parameters
                                            .push(quote! { #field_name: &solana_pubkey::Pubkey });
                                        expressions.push(quote! { #field_name.as_ref() });
                                    }
                                }
                                _ => {
                                    parameters.push(quote! { #field_name: &solana_pubkey::Pubkey });
                                    expressions.push(quote! { #field_name.as_ref() });
                                }
                            }
                        }
                    }
                    syn::Expr::MethodCall(method_call) => {
                        if let syn::Expr::Field(field_expr) = &*method_call.receiver {
                            if let syn::Member::Named(field_name) = &field_expr.member {
                                if let syn::Expr::Path(path) = &*field_expr.base {
                                    if let Some(segment) = path.path.segments.first() {
                                        if segment.ident == "data" {
                                            if let Some(data_spec) = instruction_data
                                                .iter()
                                                .find(|d| d.field_name == *field_name)
                                            {
                                                let param_type = &data_spec.field_type;
                                                let param_with_ref = if is_pubkey_type(param_type) {
                                                    quote! { #field_name: &#param_type }
                                                } else {
                                                    quote! { #field_name: #param_type }
                                                };
                                                parameters.push(param_with_ref);

                                                let method_name = &method_call.method;
                                                expressions.push(
                                                    quote! { #field_name.#method_name().as_ref() },
                                                );
                                            } else {
                                                return Err(syn::Error::new_spanned(
                                                    field_name,
                                                    format!("data.{} used in seeds but no type specified", field_name),
                                                ));
                                            }
                                        } else if segment.ident == "ctx" {
                                            // ctx.field.method() -> add field as Pubkey parameter
                                            parameters.push(
                                                quote! { #field_name: &solana_pubkey::Pubkey },
                                            );
                                            let method_name = &method_call.method;
                                            expressions.push(
                                                quote! { #field_name.#method_name().as_ref() },
                                            );
                                        }
                                    }
                                }
                            }
                        } else if let syn::Expr::Path(path_expr) = &*method_call.receiver {
                            if let Some(ident) = path_expr.path.get_ident() {
                                parameters.push(quote! { #ident: &solana_pubkey::Pubkey });
                                expressions.push(quote! { #ident.as_ref() });
                            }
                        }
                    }
                    syn::Expr::Path(path_expr) => {
                        if let Some(ident) = path_expr.path.get_ident() {
                            let ident_str = ident.to_string();
                            if ident_str
                                .chars()
                                .all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit())
                            {
                                if ident_str == "LIGHT_CPI_SIGNER" {
                                    expressions.push(quote! { #ident.cpi_signer.as_ref() });
                                } else {
                                    expressions.push(quote! { #ident.as_bytes() });
                                }
                            } else {
                                parameters.push(quote! { #ident: &solana_pubkey::Pubkey });
                                expressions.push(quote! { #ident.as_ref() });
                            }
                        } else {
                            expressions.push(quote! { (#expr).as_ref() });
                        }
                    }
                    syn::Expr::Call(call_expr) => {
                        // Recursively map data.* to parameter names in function call arguments
                        fn map_client_call_arg(
                            arg: &syn::Expr,
                            instruction_data: &[InstructionDataSpec],
                            parameters: &mut Vec<TokenStream>,
                        ) -> TokenStream {
                            match arg {
                                syn::Expr::Reference(ref_expr) => {
                                    let inner = map_client_call_arg(
                                        &ref_expr.expr,
                                        instruction_data,
                                        parameters,
                                    );
                                    quote! { &#inner }
                                }
                                syn::Expr::Field(field_expr) => {
                                    if let syn::Member::Named(field_name) = &field_expr.member {
                                        if let syn::Expr::Path(path) = &*field_expr.base {
                                            if let Some(segment) = path.path.segments.first() {
                                                if segment.ident == "data" {
                                                    // Add parameter if needed
                                                    if let Some(data_spec) = instruction_data
                                                        .iter()
                                                        .find(|d| d.field_name == *field_name)
                                                    {
                                                        let param_type = &data_spec.field_type;
                                                        let param_with_ref =
                                                            if is_pubkey_type(param_type) {
                                                                quote! { #field_name: &#param_type }
                                                            } else {
                                                                quote! { #field_name: #param_type }
                                                            };
                                                        if !parameters.iter().any(|p| {
                                                            p.to_string()
                                                                .contains(&field_name.to_string())
                                                        }) {
                                                            parameters.push(param_with_ref);
                                                        }
                                                    }
                                                    return quote! { #field_name };
                                                } else if segment.ident == "ctx" {
                                                    // ctx.field -> add as Pubkey parameter
                                                    if !parameters.iter().any(|p| {
                                                        p.to_string()
                                                            .contains(&field_name.to_string())
                                                    }) {
                                                        parameters.push(quote! { #field_name: &solana_pubkey::Pubkey });
                                                    }
                                                    return quote! { #field_name };
                                                }
                                            }
                                        }
                                    }
                                    quote! { #field_expr }
                                }
                                syn::Expr::MethodCall(method_call) => {
                                    let receiver = map_client_call_arg(
                                        &method_call.receiver,
                                        instruction_data,
                                        parameters,
                                    );
                                    let method = &method_call.method;
                                    let args: Vec<_> = method_call
                                        .args
                                        .iter()
                                        .map(|a| {
                                            map_client_call_arg(a, instruction_data, parameters)
                                        })
                                        .collect();
                                    quote! { (#receiver).#method(#(#args),*) }
                                }
                                syn::Expr::Call(nested_call) => {
                                    let func = &nested_call.func;
                                    let args: Vec<_> = nested_call
                                        .args
                                        .iter()
                                        .map(|a| {
                                            map_client_call_arg(a, instruction_data, parameters)
                                        })
                                        .collect();
                                    quote! { (#func)(#(#args),*) }
                                }
                                _ => quote! { #arg },
                            }
                        }

                        let mut mapped_args: Vec<TokenStream> = Vec::new();
                        for arg in &call_expr.args {
                            let mapped =
                                map_client_call_arg(arg, instruction_data, &mut parameters);
                            mapped_args.push(mapped);
                        }
                        let func = &call_expr.func;
                        expressions.push(quote! { (#func)(#(#mapped_args),*).as_ref() });
                    }
                    syn::Expr::Reference(ref_expr) => {
                        let (ref_params, ref_exprs) =
                            analyze_seed_spec_for_client_expr(&ref_expr.expr, instruction_data)?;
                        parameters.extend(ref_params);
                        if let Some(first_expr) = ref_exprs.first() {
                            expressions.push(quote! { (#first_expr).as_ref() });
                        }
                    }
                    _ => {
                        expressions.push(quote! { (#expr).as_ref() });
                    }
                }
            }
        }
    }

    Ok((parameters, expressions))
}

#[inline(never)]
fn analyze_seed_spec_for_client_expr(
    expr: &syn::Expr,
    instruction_data: &[InstructionDataSpec],
) -> Result<(Vec<TokenStream>, Vec<TokenStream>)> {
    let mut parameters = Vec::new();
    let mut expressions = Vec::new();

    match expr {
        syn::Expr::Field(field_expr) => {
            if let syn::Member::Named(field_name) = &field_expr.member {
                if let syn::Expr::Field(nested_field) = &*field_expr.base {
                    if let syn::Member::Named(base_name) = &nested_field.member {
                        if base_name == "accounts" {
                            parameters.push(quote! { #field_name: &solana_pubkey::Pubkey });
                            expressions.push(quote! { #field_name });
                        } else if base_name == "data" {
                            // Use declared instruction_data types to determine parameter type
                            if let Some(data_spec) = instruction_data
                                .iter()
                                .find(|d| d.field_name == *field_name)
                            {
                                let param_type = &data_spec.field_type;
                                let param_with_ref = if is_pubkey_type(param_type) {
                                    quote! { #field_name: &#param_type }
                                } else {
                                    quote! { #field_name: #param_type }
                                };
                                parameters.push(param_with_ref);
                                expressions.push(quote! { #field_name });
                            } else {
                                return Err(syn::Error::new_spanned(
                                    field_name,
                                    format!(
                                        "data.{} used in seeds but no type specified",
                                        field_name
                                    ),
                                ));
                            }
                        }
                    }
                } else if let syn::Expr::Path(path) = &*field_expr.base {
                    if let Some(segment) = path.path.segments.first() {
                        if segment.ident == "ctx" {
                            parameters.push(quote! { #field_name: &solana_pubkey::Pubkey });
                            expressions.push(quote! { #field_name });
                        }
                    }
                }
            }
        }
        syn::Expr::MethodCall(method_call) => {
            let (recv_params, _) =
                analyze_seed_spec_for_client_expr(&method_call.receiver, instruction_data)?;
            parameters.extend(recv_params);
        }
        syn::Expr::Call(call_expr) => {
            for arg in &call_expr.args {
                let (arg_params, _) = analyze_seed_spec_for_client_expr(arg, instruction_data)?;
                parameters.extend(arg_params);
            }
        }
        syn::Expr::Reference(ref_expr) => {
            let (ref_params, _) =
                analyze_seed_spec_for_client_expr(&ref_expr.expr, instruction_data)?;
            parameters.extend(ref_params);
        }
        syn::Expr::Path(path_expr) => {
            if let Some(ident) = path_expr.path.get_ident() {
                let name = ident.to_string();
                if !(name == "ctx"
                    || name == "data"
                    || name
                        .chars()
                        .all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit()))
                {
                    parameters.push(quote! { #ident: &solana_pubkey::Pubkey });
                }
            }
        }
        _ => {}
    }

    Ok((parameters, expressions))
}

fn camel_to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_lowercase().next().unwrap());
    }
    result
}

fn is_pubkey_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let type_name = segment.ident.to_string();
            type_name == "Pubkey" || type_name.contains("Pubkey")
        } else {
            false
        }
    } else {
        false
    }
}
