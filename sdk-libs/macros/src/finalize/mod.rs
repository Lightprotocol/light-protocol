//! RentFree derive macro for Accounts structs.
//!
//! This module provides:
//! - `#[derive(RentFree)]` - Generates the LightFinalize trait impl for accounts structs
//!   with fields marked `#[rentfree(...)]`
//!
//! Note: Instruction handlers are auto-wrapped by `#[rentfree_program]`.

mod codegen;
mod parse;

use proc_macro2::TokenStream;
use syn::DeriveInput;

pub fn derive_light_finalize(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    let parsed = parse::parse_compressible_struct(&input)?;
    Ok(codegen::generate_finalize_impl(&parsed))
}
