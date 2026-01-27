//! Seed expression conversion and derivation utilities.
//!
//! This module provides reusable utilities for:
//! - Converting SeedElement to TokenStream expressions
//! - Generating seed derivation code
//! - Extracting context fields from seed specifications

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use super::parsing::SeedElement;
use crate::light_pdas::shared_utils::is_constant_identifier;

// =============================================================================
// SEED EXPRESSION CONVERSION
// =============================================================================

/// Configuration for seed expression conversion.
#[derive(Clone, Debug, Default)]
pub struct SeedConversionConfig {
    /// Handle LIGHT_CPI_SIGNER specially with .cpi_signer.as_ref()
    pub handle_light_cpi_signer: bool,
    /// Map ctx.* to destructured field names (use field directly instead of ctx.field)
    pub map_ctx_to_destructured: bool,
}

impl SeedConversionConfig {
    /// Config for ctoken seed provider (destructures ctx fields).
    pub fn for_ctoken_provider() -> Self {
        Self {
            handle_light_cpi_signer: true,
            map_ctx_to_destructured: true,
        }
    }
}

/// Convert a SeedElement to a TokenStream representing the seed reference expression.
pub fn seed_element_to_ref_expr(seed: &SeedElement, config: &SeedConversionConfig) -> TokenStream {
    match seed {
        SeedElement::Literal(lit) => {
            let value = lit.value();
            quote! { #value.as_bytes() }
        }
        SeedElement::Expression(expr) => {
            // Handle byte string literals
            if let syn::Expr::Lit(lit_expr) = &**expr {
                if let syn::Lit::ByteStr(byte_str) = &lit_expr.lit {
                    let bytes = byte_str.value();
                    return quote! { &[#(#bytes),*] };
                }
            }

            // Handle uppercase constants (paths are already fully qualified by
            // convert_classified_to_seed_elements, so both single-segment and
            // multi-segment paths are handled uniformly here).
            // Skip type-qualified paths (qself) like <T as Trait>::CONST --
            // those need the full expression to preserve the type qualification.
            if let syn::Expr::Path(path_expr) = &**expr {
                if path_expr.qself.is_none() {
                    if let Some(last_seg) = path_expr.path.segments.last() {
                        if is_constant_identifier(&last_seg.ident.to_string()) {
                            let path = &path_expr.path;
                            if config.handle_light_cpi_signer && last_seg.ident == "LIGHT_CPI_SIGNER"
                            {
                                return quote! { #path.cpi_signer.as_ref() };
                            }
                            return quote! { { let __seed: &[u8] = #path.as_ref(); __seed } };
                        }
                    }
                }
            }

            // Handle ctx.accounts.field or ctx.field
            if config.map_ctx_to_destructured {
                if let Some(field_name) = extract_ctx_field_name(expr) {
                    return quote! { #field_name.as_ref() };
                }
            }

            // Fallback - wrap in type-annotated block to ensure type inference succeeds
            quote! { { let __seed: &[u8] = (#expr).as_ref(); __seed } }
        }
    }
}

/// Extract the field name from a ctx.field or ctx.accounts.field expression.
///
/// Uses the visitor-based FieldExtractor for clean pattern matching.
fn extract_ctx_field_name(expr: &syn::Expr) -> Option<Ident> {
    let fields = super::visitors::FieldExtractor::ctx_fields(&[]).extract(expr);
    fields.into_iter().next()
}

// =============================================================================
// SEED DERIVATION GENERATION
// =============================================================================

/// Generate the body of a seed function that computes a PDA address.
///
/// Returns code that:
/// 1. Builds seed_values Vec
/// 2. Computes PDA with find_program_address
/// 3. Appends bump to seeds
/// 4. Returns (seeds_vec, pda)
pub fn generate_seed_derivation_body(
    seed_expressions: &[TokenStream],
    program_id_expr: TokenStream,
) -> TokenStream {
    let seed_count = seed_expressions.len();
    quote! {
        let mut seed_values = Vec::with_capacity(#seed_count + 1);
        #(
            seed_values.push((#seed_expressions).to_vec());
        )*
        let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(&seed_slices, #program_id_expr);
        seed_values.push(vec![bump]);
        (seed_values, pda)
    }
}

/// Build set of ctx field names from identifiers.
pub fn ctx_fields_to_set(fields: &[Ident]) -> HashSet<String> {
    fields.iter().map(|f| f.to_string()).collect()
}
