//! Utility functions for instruction decoder macros.
//!
//! This module provides common utilities for:
//! - Case conversion (snake_case, PascalCase)
//! - Anchor discriminator computation
//! - Error handling helpers
//! - Program ID validation

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use sha2::{Digest, Sha256};

/// Convert a `syn::Result<TokenStream2>` to `proc_macro::TokenStream`.
///
/// This centralizes error handling for macro entry points, ensuring compile
/// errors are properly surfaced to the user with correct span information.
///
/// # Example
///
/// ```ignore
/// #[proc_macro_derive(MyMacro)]
/// pub fn my_macro(input: TokenStream) -> TokenStream {
///     into_token_stream(my_macro_impl(input.into()))
/// }
/// ```
pub(crate) fn into_token_stream(result: syn::Result<TokenStream2>) -> TokenStream {
    result.unwrap_or_else(|err| err.to_compile_error()).into()
}

/// Convert PascalCase to snake_case using heck for proper acronym handling.
///
/// Uses heck's ToSnakeCase to match Anchor's discriminator calculation behavior,
/// which groups consecutive capitals as acronyms (e.g., "CreateATA" -> "create_ata").
///
/// # Examples
///
/// ```ignore
/// assert_eq!(to_snake_case("CreateRecord"), "create_record");
/// assert_eq!(to_snake_case("CreateATA"), "create_ata");
/// assert_eq!(to_snake_case("Init"), "init");
/// ```
pub(crate) fn to_snake_case(name: &str) -> String {
    use heck::ToSnakeCase;
    name.to_snake_case()
}

/// Convert snake_case to PascalCase.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(to_pascal_case("create_record"), "CreateRecord");
/// assert_eq!(to_pascal_case("init"), "Init");
/// ```
pub(crate) fn to_pascal_case(name: &str) -> String {
    name.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Compute Anchor-style instruction discriminator.
///
/// Anchor discriminators are the first 8 bytes of SHA256("global:<instruction_name>")
/// where instruction_name is in snake_case.
///
/// # Examples
///
/// ```ignore
/// let disc = compute_anchor_discriminator("create_record");
/// assert_eq!(disc.len(), 8);
/// ```
pub(crate) fn compute_anchor_discriminator(instruction_name: &str) -> [u8; 8] {
    let preimage = format!("global:{}", instruction_name);
    let hash = Sha256::digest(preimage.as_bytes());
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&hash[..8]);
    discriminator
}

/// Convert a PascalCase name to human-readable format with spaces.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(pascal_to_display("CreateRecord"), "Create Record");
/// assert_eq!(pascal_to_display("MyProgram"), "My Program");
/// ```
pub(crate) fn pascal_to_display(name: &str) -> String {
    let mut result = String::new();
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            result.push(' ');
        }
        result.push(c);
    }
    result
}

/// Parse a base58-encoded program ID string and return token stream for byte array.
///
/// # Errors
///
/// Returns an error if the string is not valid base58 or doesn't decode to 32 bytes.
pub(crate) fn parse_program_id_bytes(id_str: &str, span: Span) -> syn::Result<TokenStream2> {
    let bytes = bs58::decode(id_str)
        .into_vec()
        .map_err(|_| syn::Error::new(span, "invalid base58 program ID"))?;

    if bytes.len() != 32 {
        return Err(syn::Error::new(
            span,
            format!("program ID must be 32 bytes, got {}", bytes.len()),
        ));
    }

    Ok(quote! { [#(#bytes),*] })
}

/// Validate discriminator size.
///
/// Valid sizes are:
/// - 1 byte: Native programs with simple instruction indices
/// - 4 bytes: System-style programs (little-endian u32)
/// - 8 bytes: Anchor programs (SHA256 prefix)
///
/// # Errors
///
/// Returns an error if size is not 1, 4, or 8.
pub(crate) fn validate_discriminator_size(size: u8, span: Span) -> syn::Result<()> {
    if ![1, 4, 8].contains(&size) {
        return Err(syn::Error::new(
            span,
            "discriminator_size must be 1 (native), 4 (system), or 8 (Anchor)",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("CreateRecord"), "create_record");
        assert_eq!(to_snake_case("UpdateScore"), "update_score");
        assert_eq!(to_snake_case("Init"), "init");
        // heck properly handles acronyms - consecutive capitals stay grouped
        assert_eq!(to_snake_case("CreateATA"), "create_ata");
        assert_eq!(to_snake_case("HTTPHandler"), "http_handler");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("create_record"), "CreateRecord");
        assert_eq!(to_pascal_case("update_score"), "UpdateScore");
        assert_eq!(to_pascal_case("init"), "Init");
    }

    #[test]
    fn test_compute_anchor_discriminator() {
        let disc = compute_anchor_discriminator("create_record");
        assert_eq!(disc.len(), 8);
        assert!(disc.iter().any(|&b| b != 0));

        // Same input should give same output (deterministic)
        let disc2 = compute_anchor_discriminator("create_record");
        assert_eq!(disc, disc2);
    }

    #[test]
    fn test_pascal_to_display() {
        assert_eq!(pascal_to_display("CreateRecord"), "Create Record");
        assert_eq!(pascal_to_display("Init"), "Init");
        assert_eq!(pascal_to_display("MyProgram"), "My Program");
    }

    #[test]
    fn test_validate_discriminator_size() {
        assert!(validate_discriminator_size(1, Span::call_site()).is_ok());
        assert!(validate_discriminator_size(4, Span::call_site()).is_ok());
        assert!(validate_discriminator_size(8, Span::call_site()).is_ok());
        assert!(validate_discriminator_size(2, Span::call_site()).is_err());
        assert!(validate_discriminator_size(16, Span::call_site()).is_err());
    }
}
