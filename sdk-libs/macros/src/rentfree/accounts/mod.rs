//! Rent-free accounts derive macro implementation.
//!
//! This module provides `#[derive(RentFree)]` which generates:
//! - `LightPreInit` trait implementation for pre-instruction compression setup
//! - `LightFinalize` trait implementation for post-instruction cleanup
//! - Supports rent-free PDAs, rent-free token accounts, and light mints
//!
//! Module structure:
//! - `parse.rs` - Parsing #[rentfree] and #[light_mint] attributes
//! - `pda.rs` - PDA block code generation
//! - `light_mint.rs` - Mint action invocation code generation
//! - `derive.rs` - Orchestration layer that wires everything together

mod builder;
mod derive;
mod light_mint;
mod parse;
mod pda;

use proc_macro2::TokenStream;
use syn::DeriveInput;

pub fn derive_rentfree(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    derive::derive_rentfree(&input)
}
