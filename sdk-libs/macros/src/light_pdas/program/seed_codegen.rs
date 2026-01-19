//! Seed provider generation for PDA and Light Token accounts.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Result;

use super::{
    instructions::{InstructionDataSpec, TokenSeedSpec},
    seed_utils::{generate_seed_derivation_body, seed_element_to_ref_expr, SeedConversionConfig},
    variant_enum::extract_ctx_fields_from_token_spec,
    visitors::{classify_seed, generate_client_seed_code},
};

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
        let token_seed_refs: Vec<TokenStream> = spec
            .seeds
            .iter()
            .map(|s| seed_element_to_ref_expr(s, &config))
            .collect();

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
            let auth_seed_refs: Vec<TokenStream> = authority_seeds
                .iter()
                .map(|s| seed_element_to_ref_expr(s, &config))
                .collect();

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
                        LightInstructionError::MissingSeedAccount.into()
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
