//! LightFinalize derive macro and light_instruction attribute macro.
//!
//! This module provides:
//! - `#[derive(LightFinalize)]` - Generates the LightFinalize trait impl for accounts structs
//!   with fields marked `#[compressible(...)]`
//! - `#[light_instruction(params)]` - Attribute macro that auto-calls light_finalize at end of handler

mod codegen;
pub mod instruction;
mod parse;

use proc_macro2::TokenStream;
use syn::DeriveInput;

pub fn derive_light_finalize(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    let parsed = parse::parse_compressible_struct(&input)?;
    Ok(codegen::generate_finalize_impl(&parsed))
}
