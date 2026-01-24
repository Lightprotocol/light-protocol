//! Orchestration layer for LightAccounts derive macro.
//!
//! This module coordinates code generation by combining:
//! - PDA block generation from `pda.rs`
//! - Mint action invocation from `mint.rs`
//! - Token account creation from `token.rs`
//! - Parsing results from `parse.rs`
//!
//! Design: ALL account creation happens in pre_init (before instruction handler)
//!
//! Account types handled:
//! - PDAs (compressed accounts)
//! - Mints (compressed mints - CREATE + DECOMPRESS atomically)
//! - Token accounts (vaults for transfers)
//! - ATAs (associated token accounts)
//!
//! Flow:
//! 1. Pre-init: ALL account creation executes here
//!    a. Write PDAs to CPI context
//!    b. Create mints with decompress + CPI context
//!    c. Create token accounts (vaults)
//!    d. Create ATAs
//! 2. Instruction body: All accounts available for use (transfers, minting, etc.)
//! 3. Finalize: No-op (all work done in pre_init)

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use super::builder::LightAccountsBuilder;

/// Main orchestration - shows the high-level flow clearly.
pub(crate) fn derive_light_accounts(input: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let builder = LightAccountsBuilder::parse(input)?;
    builder.validate()?;

    // No instruction args = no-op impls (backwards compatibility)
    if !builder.has_instruction_args() {
        return builder.generate_noop_impls();
    }

    // Generate pre_init body for ALL account types (PDAs, mints, token accounts, ATAs)
    // ALL account creation happens here so accounts are available during instruction handler
    let pre_init = builder.generate_pre_init_all()?;

    // Generate trait implementations
    let pre_init_impl = builder.generate_pre_init_impl(pre_init)?;

    // Finalize is now a no-op - all account creation happens in pre_init
    let finalize_body = quote! { Ok(()) };
    let finalize_impl = builder.generate_finalize_impl(finalize_body)?;

    Ok(quote! {
        #pre_init_impl
        #finalize_impl
    })
}
