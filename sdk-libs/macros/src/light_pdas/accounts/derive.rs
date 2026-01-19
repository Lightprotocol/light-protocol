//! Orchestration layer for LightAccounts derive macro.
//!
//! This module coordinates code generation by combining:
//! - PDA block generation from `pda.rs`
//! - Mint action invocation from `mint.rs`
//! - Parsing results from `parse.rs`
//!
//! Design for mints:
//! - At mint init, we CREATE + DECOMPRESS atomically
//! - After init, the Mint should always be in decompressed/"hot" state
//!
//! Flow for PDAs + mints:
//! 1. Pre-init: ALL compression logic executes here
//!    a. Write PDAs to CPI context
//!    b. Invoke mint_action with decompress + CPI context
//!    c. Mint is now "hot" and usable
//! 2. Instruction body: Can use hot Mint (mintTo, transfers, etc.)
//! 3. Finalize: No-op (all work done in pre_init)

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use super::builder::LightAccountsBuilder;

/// Main orchestration - shows the high-level flow clearly.
pub(super) fn derive_light_accounts(input: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let builder = LightAccountsBuilder::parse(input)?;
    builder.validate()?;

    // No instruction args = no-op impls (backwards compatibility)
    if !builder.has_instruction_args() {
        return builder.generate_noop_impls();
    }

    // Generate pre_init body based on what fields we have
    let pre_init = if builder.has_pdas() && builder.has_mints() {
        builder.generate_pre_init_pdas_and_mints()?
    } else if builder.has_mints() {
        builder.generate_pre_init_mints_only()?
    } else if builder.has_pdas() {
        builder.generate_pre_init_pdas_only()?
    } else {
        quote! { Ok(false) }
    };

    // Generate trait implementations
    let pre_init_impl = builder.generate_pre_init_impl(pre_init)?;
    let finalize_impl = builder.generate_finalize_impl(quote! { Ok(()) })?;

    Ok(quote! {
        #pre_init_impl
        #finalize_impl
    })
}
