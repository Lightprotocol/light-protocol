//! Seed provider generation for PDA and Light Token accounts.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result};

use super::instructions::{InstructionDataSpec, SeedElement, TokenSeedSpec};
use super::seed_utils::{generate_seed_derivation_body, seed_element_to_ref_expr, SeedConversionConfig};
use super::variant_enum::extract_ctx_fields_from_token_spec;
use crate::rentfree::shared_utils::is_constant_identifier;
use crate::rentfree::traits::utils::is_pubkey_type;

/// Helper to add a Pubkey parameter and its .as_ref() expression.
/// This is the default fallback for ctx.accounts.field and similar patterns.
#[inline]
fn push_pubkey_param(
    field_name: &syn::Ident,
    parameters: &mut Vec<TokenStream>,
    expressions: &mut Vec<TokenStream>,
) {
    parameters.push(quote! { #field_name: &solana_pubkey::Pubkey });
    expressions.push(quote! { #field_name.as_ref() });
}


/// Phase 8: Generate TokenSeedProvider impl that uses self.field instead of ctx.accounts.field
pub fn generate_ctoken_seed_provider_implementation(
    token_seeds: &[TokenSeedSpec],
) -> Result<TokenStream> {
    let mut get_seeds_match_arms = Vec::new();
    let mut get_authority_seeds_match_arms = Vec::new();

    let config = SeedConversionConfig::for_ctoken_provider();

    for spec in token_seeds {
        let variant_name = &spec.variant;
        let ctx_fields = extract_ctx_fields_from_token_spec(spec);

        // Build match pattern with destructuring if there are ctx fields
        let pattern = if ctx_fields.is_empty() {
            quote! { TokenAccountVariant::#variant_name }
        } else {
            let field_names: Vec<_> = ctx_fields.iter().collect();
            quote! { TokenAccountVariant::#variant_name { #(#field_names,)* } }
        };

        // Build seed refs for get_seeds - use self.field directly for ctx.* seeds
        let token_seed_refs: Vec<TokenStream> = spec.seeds.iter().map(|s| seed_element_to_ref_expr(s, &config)).collect();

        let get_seeds_arm = quote! {
            #pattern => {
                let seeds: &[&[u8]] = &[#(#token_seed_refs),*];
                let (token_account_pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, program_id);
                let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                seeds_vec.extend(seeds.iter().map(|s| s.to_vec()));
                seeds_vec.push(vec![bump]);
                Ok((seeds_vec, token_account_pda))
            }
        };
        get_seeds_match_arms.push(get_seeds_arm);

        // Build authority seeds
        if let Some(authority_seeds) = &spec.authority {
            let auth_seed_refs: Vec<TokenStream> = authority_seeds.iter().map(|s| seed_element_to_ref_expr(s, &config)).collect();

            let authority_arm = quote! {
                #pattern => {
                    let seeds: &[&[u8]] = &[#(#auth_seed_refs),*];
                    let (authority_pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, program_id);
                    let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                    seeds_vec.extend(seeds.iter().map(|s| s.to_vec()));
                    seeds_vec.push(vec![bump]);
                    Ok((seeds_vec, authority_pda))
                }
            };
            get_authority_seeds_match_arms.push(authority_arm);
        } else {
            let authority_arm = quote! {
                #pattern => {
                    Err(solana_program_error::ProgramError::Custom(
                        RentFreeInstructionError::MissingSeedAccount.into()
                    ))
                }
            };
            get_authority_seeds_match_arms.push(authority_arm);
        }
    }

    // Phase 8: New trait signature - no ctx/accounts parameter needed
    Ok(quote! {
        impl light_sdk::compressible::TokenSeedProvider for TokenAccountVariant {
            fn get_seeds(
                &self,
                program_id: &solana_pubkey::Pubkey,
            ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), solana_program_error::ProgramError> {
                match self {
                    #(#get_seeds_match_arms)*
                }
            }

            fn get_authority_seeds(
                &self,
                program_id: &solana_pubkey::Pubkey,
            ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), solana_program_error::ProgramError> {
                match self {
                    #(#get_authority_seeds_match_arms)*
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

            let fn_body =
                generate_seed_derivation_body(&seed_expressions, quote! { &crate::ID });
            let function = quote! {
                pub fn #function_name(#(#parameters),*) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
                    #fn_body
                }
            };
            functions.push(function);
        }
    }

    if let Some(token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            let variant_name = &spec.variant;

            let function_name =
                format_ident!("get_{}_seeds", variant_name.to_string().to_lowercase());

            let (parameters, seed_expressions) =
                analyze_seed_spec_for_client(spec, instruction_data)?;

            let fn_body =
                generate_seed_derivation_body(&seed_expressions, quote! { &crate::ID });
            let function = quote! {
                pub fn #function_name(#(#parameters),*) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
                    #fn_body
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
                    seeds: syn::punctuated::Punctuated::new(),
                    authority: None,
                };

                for auth_seed in authority_seeds {
                    authority_spec.seeds.push(auth_seed.clone());
                }

                let (auth_parameters, auth_seed_expressions) =
                    analyze_seed_spec_for_client(&authority_spec, instruction_data)?;

                let (fn_params, fn_body) = if auth_parameters.is_empty() {
                    (
                        quote! { _program_id: &solana_pubkey::Pubkey },
                        generate_seed_derivation_body(&auth_seed_expressions, quote! { _program_id }),
                    )
                } else {
                    (
                        quote! { #(#auth_parameters),* },
                        generate_seed_derivation_body(&auth_seed_expressions, quote! { &crate::ID }),
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
                            // Check for data.field pattern which uses instruction_data types
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
                                            expressions.push(quote! { #field_name.as_ref() });
                                            continue;
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
                            }
                            // Default: ctx.accounts.field, ctx.field, or other field patterns
                            push_pubkey_param(field_name, &mut parameters, &mut expressions);
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
                    syn::Expr::Lit(lit_expr) => {
                        // Handle byte string literals: b"seed" -> use directly
                        if let syn::Lit::ByteStr(byte_str) = &lit_expr.lit {
                            let bytes = byte_str.value();
                            expressions.push(quote! { &[#(#bytes),*] });
                        }
                    }
                    syn::Expr::Path(path_expr) => {
                        if let Some(ident) = path_expr.path.get_ident() {
                            let ident_str = ident.to_string();
                            if is_constant_identifier(&ident_str) {
                                if ident_str == "LIGHT_CPI_SIGNER" {
                                    expressions.push(quote! { crate::#ident.cpi_signer.as_ref() });
                                } else {
                                    // Use crate:: prefix and explicit type annotation
                                    expressions.push(quote! { { let __seed: &[u8] = crate::#ident.as_ref(); __seed } });
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
                if !(name == "ctx" || name == "data" || is_constant_identifier(&name)) {
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
