//! Rent-free accounts derive macro implementation.
//!
//! This module provides `#[derive(RentFree)]` which generates:
//! - `LightPreInit` trait implementation for pre-instruction compression setup
//! - `LightFinalize` trait implementation for post-instruction cleanup
//! - Supports rent-free PDAs, rent-free token accounts, and light mints

mod codegen;
mod light_mint;
mod parse;

use proc_macro2::TokenStream;
use syn::DeriveInput;

pub fn derive_rentfree(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    let parsed = parse::parse_rentfree_struct(&input)?;
    codegen::generate_rentfree_impl(&parsed)
}
