//! Light accounts derive macro implementation.
//!
//! This module provides `#[derive(LightAccounts)]` which generates:
//! - `LightPreInit` trait implementation for pre-instruction compression setup
//! - `LightFinalize` trait implementation for post-instruction cleanup
//! - Per-field variant structs and trait implementations
//! - Supports Light PDAs, Light token accounts, and light mints
//!
//! Module structure:
//! - `light_account.rs` - Unified parsing for #[light_account(init, ...)] attributes
//! - `parse.rs` - Struct-level parsing and field classification
//! - `pda.rs` - PDA block code generation
//! - `mint.rs` - Mint action invocation code generation
//! - `variant.rs` - Per-field variant struct and trait generation
//! - `derive.rs` - Orchestration layer that wires everything together

mod builder;
mod derive;
mod light_account;
mod mint;
mod parse;
mod pda;
mod token;
mod validation;
mod variant;

use proc_macro2::TokenStream;
use syn::DeriveInput;

pub fn derive_light_accounts(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    derive::derive_light_accounts(&input)
}
