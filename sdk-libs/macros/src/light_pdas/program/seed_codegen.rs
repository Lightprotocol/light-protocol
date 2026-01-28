//! Seed provider generation for PDA and Light Token accounts.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Result;

use super::{
    instructions::{InstructionDataSpec, TokenSeedSpec},
    seed_utils::{generate_seed_derivation_body, seed_element_to_ref_expr, SeedConversionConfig},
    visitors::{classify_seed, generate_client_seed_code},
};

/// Generate seed-related helper functions for token variants.
///
/// Currently generates client seed functions only. The legacy TokenSeedProvider
/// trait impls have been removed; seed derivation is now handled directly
/// via `LightAccountVariantTrait` impls generated in `variant_enum.rs`.
pub fn generate_ctoken_seed_provider_implementation(
    _token_seeds: &[TokenSeedSpec],
) -> Result<TokenStream> {
    // TokenSeedProvider is legacy - seed derivation is now handled by
    // LightAccountVariantTrait impls generated in variant_enum.rs
    Ok(quote! {})
}

#[inline(never)]
pub fn generate_client_seed_functions(
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

            let fn_body = generate_seed_derivation_body(&seed_expressions, quote! { &crate::ID });
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

            let fn_body = generate_seed_derivation_body(&seed_expressions, quote! { &crate::ID });
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
                    inner_type: spec.inner_type.clone(),
                    is_zero_copy: spec.is_zero_copy,
                };

                for auth_seed in authority_seeds {
                    authority_spec.seeds.push(auth_seed.clone());
                }

                let (auth_parameters, auth_seed_expressions) =
                    analyze_seed_spec_for_client(&authority_spec, instruction_data)?;

                let (fn_params, fn_body) = if auth_parameters.is_empty() {
                    (
                        quote! { _program_id: &solana_pubkey::Pubkey },
                        generate_seed_derivation_body(
                            &auth_seed_expressions,
                            quote! { _program_id },
                        ),
                    )
                } else {
                    (
                        quote! { #(#auth_parameters),* },
                        generate_seed_derivation_body(
                            &auth_seed_expressions,
                            quote! { &crate::ID },
                        ),
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

/// Analyze a seed spec and generate client function parameters and seed expressions.
///
/// Uses the classification-based approach: first classify each seed, then generate code.
/// This separates "what kind of seed is this?" from "what code to generate?".
#[inline(never)]
fn analyze_seed_spec_for_client(
    spec: &TokenSeedSpec,
    instruction_data: &[InstructionDataSpec],
) -> Result<(Vec<TokenStream>, Vec<TokenStream>)> {
    let mut parameters = Vec::new();
    let mut expressions = Vec::new();
    let mut seen_params = HashSet::new();

    for seed in &spec.seeds {
        // Phase 1: Classification
        let info = classify_seed(seed)?;

        // Phase 2: Code generation (modifies parameters and expressions in place)
        generate_client_seed_code(
            &info,
            instruction_data,
            &mut seen_params,
            &mut parameters,
            &mut expressions,
        )?;
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
