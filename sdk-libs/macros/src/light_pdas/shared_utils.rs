//! Shared utilities for rentfree macro implementation.
//!
//! This module provides common utility functions used across multiple files:
//! - Constant identifier detection (SCREAMING_SNAKE_CASE)
//! - Expression identifier extraction
//! - MetaExpr for darling attribute parsing

use darling::FromMeta;
use quote::format_ident;
use syn::{Expr, Ident, Type};

// ============================================================================
// Type path helpers for preserving full type paths in code generation
// ============================================================================

/// Ensures a type path is fully qualified with `crate::` prefix.
/// For types that are already qualified (crate::, super::, self::, or absolute ::),
/// returns them unchanged. For bare types like `MyRecord`, returns `crate::MyRecord`.
///
/// This ensures generated code can reference types regardless of what imports
/// are in scope at the generation site.
pub fn qualify_type_with_crate(ty: &Type) -> Type {
    if let Type::Path(type_path) = ty {
        // Check if already qualified
        if let Some(first_seg) = type_path.path.segments.first() {
            let first_str = first_seg.ident.to_string();
            // Already qualified with crate, super, self, or starts with ::
            if first_str == "crate" || first_str == "super" || first_str == "self" {
                return ty.clone();
            }
        }
        // Check for absolute path (starts with ::)
        if type_path.path.leading_colon.is_some() {
            return ty.clone();
        }

        // Prepend crate:: to the path
        let mut qualified_path = type_path.clone();
        let crate_segment: syn::PathSegment = syn::parse_quote!(crate);
        qualified_path.path.segments.insert(0, crate_segment);
        Type::Path(qualified_path)
    } else {
        ty.clone()
    }
}

/// Creates a packed type path from an original type.
/// For `crate::module::MyRecord` returns `crate::module::PackedMyRecord`
/// For `MyRecord` returns `crate::PackedMyRecord` (qualified and packed)
///
/// First qualifies the type with `crate::`, then prepends "Packed" to the terminal type name.
pub fn make_packed_type(ty: &Type) -> Option<Type> {
    // First qualify the type
    let qualified = qualify_type_with_crate(ty);

    if let Type::Path(type_path) = &qualified {
        let mut packed_path = type_path.clone();
        if let Some(last_seg) = packed_path.path.segments.last_mut() {
            let packed_name = format_ident!("Packed{}", last_seg.ident);
            last_seg.ident = packed_name;
        }
        Some(Type::Path(packed_path))
    } else {
        None
    }
}

/// Creates a simple type from an identifier (for cases where we only have variant name).
/// Converts `MyRecord` Ident to `MyRecord` Type.
pub fn ident_to_type(ident: &Ident) -> Type {
    let path: syn::Path = ident.clone().into();
    Type::Path(syn::TypePath { qself: None, path })
}

// ============================================================================
// darling support for parsing Expr from attributes
// ============================================================================

/// Wrapper for syn::Expr that implements darling's FromMeta trait.
///
/// Enables darling to parse arbitrary expressions in attributes like
/// `#[light_account(init, mint, mint_signer = self.authority)]`.
#[derive(Clone)]
pub struct MetaExpr(Expr);

impl FromMeta for MetaExpr {
    fn from_expr(expr: &Expr) -> darling::Result<Self> {
        Ok(MetaExpr(expr.clone()))
    }
}

impl From<MetaExpr> for Expr {
    fn from(meta: MetaExpr) -> Expr {
        meta.0
    }
}

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
    !ident.is_empty()
        && ident
            .chars()
            .all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit())
}

/// Check if an expression is a path starting with the given base identifier.
///
/// Used to check patterns like `ctx.field` where base would be "ctx".
#[inline]
pub fn is_base_path(expr: &Expr, base: &str) -> bool {
    matches!(expr, Expr::Path(p) if p.path.segments.first().is_some_and(|s| s.ident == base))
}

/// Convert a snake_case string to PascalCase.
///
/// # Examples
/// ```ignore
/// assert_eq!(to_pascal_case("user_record"), "UserRecord");
/// assert_eq!(to_pascal_case("my_data"), "MyData");
/// assert_eq!(to_pascal_case("record"), "Record");
/// ```
pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + chars.as_str().to_lowercase().as_str()
                }
                None => String::new(),
            }
        })
        .collect()
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

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("user_record"), "UserRecord");
        assert_eq!(to_pascal_case("my_data"), "MyData");
        assert_eq!(to_pascal_case("record"), "Record");
        assert_eq!(to_pascal_case("a_b_c"), "ABC");
        assert_eq!(to_pascal_case(""), "");
    }
}
