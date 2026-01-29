//! Shared validation utilities for light account macros.
//!
//! # Validation Rules
//!
//! 1. **compression_info field position** - The `compression_info` field must be either
//!    the first or last field in the struct for efficient serialization
//!
//! 2. **Non-empty struct** - Struct must have at least one field
//!
//! 3. **Account type extraction** - Field types must be one of:
//!    - `Account<'info, T>`
//!    - `Box<Account<'info, T>>`
//!    - `AccountLoader<'info, T>`
//!    - `InterfaceAccount<'info, T>`
//!
//! 4. **No nested Box** - `Box<Box<...>>` patterns are not supported

use std::fmt;

use syn::{punctuated::Punctuated, Field, Ident, Result, Token, Type};

/// Error types for account type extraction.
#[derive(Debug)]
pub enum AccountTypeError {
    /// The type is not Account, Box<Account>, AccountLoader, or InterfaceAccount.
    WrongType { got: String },
    /// Nested Box<Box<...>> is not supported.
    NestedBox,
    /// Failed to extract inner type from generic arguments.
    ExtractionFailed,
}

impl fmt::Display for AccountTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountTypeError::WrongType { got } => write!(
                f,
                "Expected Account<'info, T>, Box<Account<'info, T>>, AccountLoader<'info, T>, \
                 or InterfaceAccount<'info, T>, but found `{}`",
                got
            ),
            AccountTypeError::NestedBox => write!(
                f,
                "Nested Box<Box<...>> is not supported. Use Box<Account<...>> instead."
            ),
            AccountTypeError::ExtractionFailed => write!(
                f,
                "Failed to extract inner type from Account/AccountLoader generic arguments"
            ),
        }
    }
}

impl AccountTypeError {
    /// Convert this error into a syn::Error at the given span.
    pub fn into_syn_error(self, span: &impl quote::ToTokens) -> syn::Error {
        syn::Error::new_spanned(span, self.to_string())
    }
}

/// Validates that the struct has a `compression_info` field as first or last field.
/// Returns `Ok(true)` if first, `Ok(false)` if last, `Err` if missing or in middle.
pub fn validate_compression_info_field(
    fields: &Punctuated<Field, Token![,]>,
    struct_name: &Ident,
) -> Result<bool> {
    let field_count = fields.len();
    if field_count == 0 {
        return Err(syn::Error::new_spanned(
            struct_name,
            "Struct must have at least one field",
        ));
    }

    let first_is_compression_info = fields
        .first()
        .and_then(|f| f.ident.as_ref())
        .is_some_and(|name| name == "compression_info");

    let last_is_compression_info = fields
        .last()
        .and_then(|f| f.ident.as_ref())
        .is_some_and(|name| name == "compression_info");

    if first_is_compression_info {
        Ok(true)
    } else if last_is_compression_info {
        Ok(false)
    } else {
        Err(syn::Error::new_spanned(
            struct_name,
            "Field 'compression_info' must be the first or last field in the struct \
             for efficient serialization. Move it to the beginning or end of your struct definition.",
        ))
    }
}

/// Get a human-readable type name from a syn::Type for error messages.
pub fn type_name(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => type_path
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        _ => "unknown".to_string(),
    }
}
