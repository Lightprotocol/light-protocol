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
