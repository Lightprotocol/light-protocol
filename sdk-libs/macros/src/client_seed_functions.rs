//! Client-side seed function generation.
//!
//! Generates public functions for deriving PDA addresses client-side.
//! These are used by TypeScript/JavaScript clients.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result};

use crate::compressible_instructions::{InstructionDataSpec, SeedElement, TokenSeedSpec};

/// Generate public client-side seed functions for external consumption.
///
/// Creates get_X_seeds() functions that clients can call to derive PDAs.
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
                /// Auto-generated client-side seed function
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

    // Generate CToken seed functions
    if let Some(token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            let variant_name = &spec.variant;

            // Skip ATA variants
            if spec.is_ata {
                continue;
            }

            let function_name =
                format_ident!("get_{}_seeds", variant_name.to_string().to_lowercase());

            let (parameters, seed_expressions) =
                analyze_seed_spec_for_client(spec, instruction_data)?;

            let seed_count = seed_expressions.len();
            let function = quote! {
                /// Auto-generated client-side CToken seed function
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

            // Generate authority seed function
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
                // If no parameters, add a dummy parameter to avoid Anchor treating this as a fallback function
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
                    /// Auto-generated authority seed function for compression signing
                    pub fn #authority_function_name(#fn_params) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
                        #fn_body
                    }
                };
                functions.push(authority_function);
            }
        }
    }

    Ok(quote! {
        /// Client-side seed derivation functions (not program instructions)
        /// These are helper functions for clients, not Anchor program instructions
        mod __client_seed_functions {
            use super::*;
            #(#functions)*
        }

        // Re-export for convenience - these are client helpers, not instructions
        pub use __client_seed_functions::*;
    })
}

/// Analyze seed specification and generate parameters + expressions for client functions.
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
                                                // TODO: check why unused.
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
                                // Special handling for LIGHT_CPI_SIGNER - use .cpi_signer field
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
                        for arg in &call_expr.args {
                            let (arg_params, _) =
                                analyze_seed_spec_for_client_expr(arg, instruction_data)?;
                            parameters.extend(arg_params);
                        }
                        expressions.push(quote! { (#expr).as_ref() });
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

/// Helper to analyze a single expression for client functions.
#[inline(never)]
fn analyze_seed_spec_for_client_expr(
    expr: &syn::Expr,
    // TODO: check why unused
    _instruction_data: &[InstructionDataSpec],
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
                analyze_seed_spec_for_client_expr(&method_call.receiver, _instruction_data)?;
            parameters.extend(recv_params);
        }
        syn::Expr::Call(call_expr) => {
            for arg in &call_expr.args {
                let (arg_params, _) = analyze_seed_spec_for_client_expr(arg, _instruction_data)?;
                parameters.extend(arg_params);
            }
        }
        syn::Expr::Reference(ref_expr) => {
            let (ref_params, _) =
                analyze_seed_spec_for_client_expr(&ref_expr.expr, _instruction_data)?;
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

/// Convert CamelCase to snake_case
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

/// Check if a type is Pubkey-like.
#[inline(never)]
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
