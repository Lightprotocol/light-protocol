//! Shared utilities for rentfree macro implementation.
//!
//! This module provides common utility functions used across multiple files:
//! - Constant identifier detection (SCREAMING_SNAKE_CASE)
//! - Expression identifier extraction

use syn::{Expr, Ident};

/// Check if an identifier string is a constant (SCREAMING_SNAKE_CASE).
///
/// Returns true if the string is non-empty and all characters are uppercase letters,
/// underscores, or ASCII digits.
///
/// # Examples
/// ```ignore
/// assert!(is_constant_identifier("MY_CONSTANT"));
/// assert!(is_constant_identifier("SEED_123"));
/// assert!(!is_constant_identifier("myVariable"));
/// assert!(!is_constant_identifier(""));
/// ```
#[inline]
pub fn is_constant_identifier(ident: &str) -> bool {
    !ident.is_empty() && ident.chars().all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit())
}

/// Extract the terminal identifier from an expression.
///
/// This handles various expression patterns:
/// - `Path`: Returns the identifier directly
/// - `Field`: Returns the field name
/// - `MethodCall`: Recursively extracts from receiver
/// - `Reference`: Recursively extracts from referenced expression
///
/// If `key_method_only` is true, only returns an identifier from MethodCall
/// expressions where the method is `key`.
#[inline]
pub fn extract_terminal_ident(expr: &Expr, key_method_only: bool) -> Option<Ident> {
    match expr {
        Expr::Path(path) => path.path.get_ident().cloned(),
        Expr::Field(field) => {
            if let syn::Member::Named(name) = &field.member {
                Some(name.clone())
            } else {
                None
            }
        }
        Expr::MethodCall(mc) => {
            if key_method_only && mc.method != "key" {
                None
            } else {
                extract_terminal_ident(&mc.receiver, key_method_only)
            }
        }
        Expr::Reference(r) => extract_terminal_ident(&r.expr, key_method_only),
        _ => None,
    }
}

/// Check if an expression is a path starting with the given base identifier.
///
/// Used to check patterns like `ctx.field` where base would be "ctx".
#[inline]
pub fn is_base_path(expr: &Expr, base: &str) -> bool {
    matches!(expr, Expr::Path(p) if p.path.segments.first().is_some_and(|s| s.ident == base))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_constant_identifier() {
        assert!(is_constant_identifier("MY_CONSTANT"));
        assert!(is_constant_identifier("SEED"));
        assert!(is_constant_identifier("SEED_123"));
        assert!(is_constant_identifier("A"));
        assert!(!is_constant_identifier("myVariable"));
        assert!(!is_constant_identifier("my_variable"));
        assert!(!is_constant_identifier("MyConstant"));
        assert!(!is_constant_identifier(""));
    }
}
