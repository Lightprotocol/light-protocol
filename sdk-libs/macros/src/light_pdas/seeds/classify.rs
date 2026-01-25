//! Seed classification logic for the simplified 3-category system.
//!
//! This module provides the core `classify_seed` function that determines
//! whether a seed expression is:
//! - `Constant`: Known at compile time
//! - `Account`: References an account field
//! - `Data`: References instruction data

use std::collections::HashSet;

use syn::{Expr, Ident};

use super::types::{Seed, SeedKind};
use crate::light_pdas::shared_utils::is_constant_identifier;

/// Classify a seed expression into one of three categories.
///
/// Classification strategy:
/// 1. Check for literals (`b"..."`, `"..."`) -> Constant
/// 2. Check for uppercase constant paths (`SEED`, `crate::SEED`) -> Constant
/// 3. Extract root field name from expression
/// 4. Check if root is an instruction argument -> Data
/// 5. Check if root is an account field -> Account
/// 6. Fallback to Constant (passthrough for complex expressions)
///
/// # Arguments
/// * `expr` - The seed expression to classify
/// * `instruction_args` - Names of instruction arguments (from `#[instruction(...)]`)
/// * `account_fields` - Names of account fields in the Accounts struct
///
/// # Returns
/// A classified `Seed` with kind, expression, and optional field name.
pub fn classify_seed(
    expr: &Expr,
    instruction_args: &HashSet<String>,
    account_fields: &HashSet<String>,
) -> syn::Result<Seed> {
    // 1. Check if it's a literal (b"...", "...", b"..."[..])
    if is_literal(expr) {
        return Ok(Seed::constant(expr.clone()));
    }

    // 2. Check if it's an uppercase constant path (SEED, crate::SEED)
    if is_constant_path(expr) {
        return Ok(Seed::constant(expr.clone()));
    }

    // 3. Try to extract root field name and build stripped expression
    if let Some((root, stripped_expr)) = extract_root_and_strip(expr) {
        let root_str = root.to_string();

        // 4. Check if root is an instruction argument -> Data
        if instruction_args.contains(&root_str) {
            // For instruction args, we need the terminal field, not the root
            // e.g., params.owner.as_ref() -> field is "owner", not "params"
            if let Some(terminal_field) = extract_terminal_data_field(expr, &root_str) {
                return Ok(Seed::data(stripped_expr, terminal_field));
            }
            // If it's a bare instruction arg (Format 2), use root as field
            return Ok(Seed::data(stripped_expr, root));
        }

        // 5. Check if root is an account field -> Account
        if account_fields.contains(&root_str) {
            return Ok(Seed::account(stripped_expr, root));
        }
    }

    // 6. Fallback: treat as Constant (passthrough)
    // This handles complex expressions like identity_seed::<12>(b"seed")
    Ok(Seed::constant(expr.clone()))
}

/// Check if expression is a literal (byte string or string).
///
/// Handles:
/// - `b"literal"`
/// - `"string"`
/// - `b"literal"[..]` (full range slice)
/// - `&b"literal"` (reference to literal)
fn is_literal(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(lit) => {
            matches!(&lit.lit, syn::Lit::ByteStr(_) | syn::Lit::Str(_))
        }
        // Handle b"literal"[..] - full range slice
        Expr::Index(idx) => {
            if let Expr::Range(range) = &*idx.index {
                if range.start.is_none() && range.end.is_none() {
                    return is_literal(&idx.expr);
                }
            }
            false
        }
        // Unwrap references
        Expr::Reference(r) => is_literal(&r.expr),
        _ => false,
    }
}

/// Check if expression is an uppercase constant path.
///
/// Handles:
/// - `SEED_PREFIX`
/// - `crate::state::SEED_CONSTANT`
///
/// Does NOT handle type-qualified paths like `<T as Trait>::CONST` (returns false).
fn is_constant_path(expr: &Expr) -> bool {
    match expr {
        Expr::Path(path) => {
            // Type-qualified paths are not simple constants
            if path.qself.is_some() {
                return false;
            }

            // Check if the last segment is uppercase (constant naming convention)
            if let Some(last_seg) = path.path.segments.last() {
                return is_constant_identifier(&last_seg.ident.to_string());
            }
            false
        }
        // Unwrap references
        Expr::Reference(r) => is_constant_path(&r.expr),
        _ => false,
    }
}

/// Extract root field name and build a stripped expression.
///
/// This function:
/// 1. Finds the root identifier in the expression chain
/// 2. Strips any prefix like `ctx.accounts.` or `params.`
///
/// | Input | Root | Stripped expr |
/// |-------|------|---------------|
/// | `ctx.accounts.authority.key().as_ref()` | `authority` | `authority.key().as_ref()` |
/// | `authority.key().as_ref()` | `authority` | `authority.key().as_ref()` |
/// | `params.owner.as_ref()` | `params` | `owner.as_ref()` |
/// | `owner.as_ref()` | `owner` | `owner.as_ref()` |
fn extract_root_and_strip(expr: &Expr) -> Option<(Ident, Expr)> {
    // First, check if this starts with ctx.accounts prefix
    if let Some(stripped) = strip_ctx_accounts_prefix(expr) {
        if let Some(root) = extract_root_ident(&stripped) {
            return Some((root, stripped));
        }
    }

    // Otherwise, get the root from the expression directly
    let root = extract_root_ident(expr)?;

    // Check if expression needs stripping (has instruction arg prefix)
    // For expressions like params.owner.as_ref(), we want to strip params
    if is_instruction_arg_rooted(expr) {
        if let Some(stripped) = strip_instruction_arg_prefix(expr) {
            return Some((root, stripped));
        }
    }

    // No stripping needed, return as-is
    Some((root, expr.clone()))
}

/// Extract the root identifier from an expression.
///
/// Returns the first non-ctx identifier in the expression chain.
fn extract_root_ident(expr: &Expr) -> Option<Ident> {
    match expr {
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                // Skip uppercase constants
                if !is_constant_identifier(&ident.to_string()) {
                    return Some(ident.clone());
                }
            }
            None
        }
        Expr::Field(field) => {
            // For field access, check if base is ctx or similar
            if is_ctx_or_accounts_base(&field.base) {
                // Return the field name (e.g., "authority" from ctx.accounts.authority)
                if let syn::Member::Named(name) = &field.member {
                    return Some(name.clone());
                }
            }
            // Otherwise recurse into base
            extract_root_ident(&field.base)
        }
        Expr::MethodCall(mc) => extract_root_ident(&mc.receiver),
        Expr::Index(idx) => extract_root_ident(&idx.expr),
        Expr::Reference(r) => extract_root_ident(&r.expr),
        _ => None,
    }
}

/// Check if expression base is `ctx` or `ctx.accounts`.
fn is_ctx_or_accounts_base(expr: &Expr) -> bool {
    match expr {
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                return ident == "ctx";
            }
            false
        }
        Expr::Field(field) => {
            if let syn::Member::Named(name) = &field.member {
                if name == "accounts" {
                    if let Expr::Path(path) = &*field.base {
                        if let Some(ident) = path.path.get_ident() {
                            return ident == "ctx";
                        }
                    }
                }
            }
            false
        }
        _ => false,
    }
}

/// Strip `ctx.accounts.` prefix from expression.
fn strip_ctx_accounts_prefix(expr: &Expr) -> Option<Expr> {
    match expr {
        Expr::Field(field) => {
            if is_ctx_or_accounts_base(&field.base) {
                // Return the field access without ctx.accounts prefix
                if let syn::Member::Named(name) = &field.member {
                    return Some(syn::parse_quote!(#name));
                }
            }
            // Recurse and rebuild
            if let Some(new_base) = strip_ctx_accounts_prefix(&field.base) {
                let member = &field.member;
                return Some(syn::parse_quote!(#new_base.#member));
            }
            None
        }
        Expr::MethodCall(mc) => {
            if let Some(new_receiver) = strip_ctx_accounts_prefix(&mc.receiver) {
                let method = &mc.method;
                let args = &mc.args;
                let turbofish = &mc.turbofish;
                return Some(if let Some(tf) = turbofish {
                    syn::parse_quote!(#new_receiver.#method #tf (#args))
                } else {
                    syn::parse_quote!(#new_receiver.#method(#args))
                });
            }
            None
        }
        Expr::Reference(r) => {
            if let Some(stripped) = strip_ctx_accounts_prefix(&r.expr) {
                return Some(if r.mutability.is_some() {
                    syn::parse_quote!(&mut #stripped)
                } else {
                    syn::parse_quote!(&#stripped)
                });
            }
            None
        }
        _ => None,
    }
}

/// Check if expression is rooted in an instruction arg pattern.
///
/// Returns true for expressions like `params.owner.as_ref()` where the root
/// follows the pattern of field access on a single identifier.
fn is_instruction_arg_rooted(expr: &Expr) -> bool {
    match expr {
        Expr::Field(field) => {
            // Check if base is a simple path (e.g., params, args, data)
            matches!(&*field.base, Expr::Path(path) if path.path.get_ident().is_some())
        }
        Expr::MethodCall(mc) => is_instruction_arg_rooted(&mc.receiver),
        Expr::Index(idx) => is_instruction_arg_rooted(&idx.expr),
        Expr::Reference(r) => is_instruction_arg_rooted(&r.expr),
        _ => false,
    }
}

/// Strip instruction arg prefix (e.g., params.) from expression.
///
/// Transforms `params.owner.as_ref()` to `owner.as_ref()`.
fn strip_instruction_arg_prefix(expr: &Expr) -> Option<Expr> {
    match expr {
        Expr::Field(field) => {
            // If base is simple path (e.g., params), return field access from member
            if let Expr::Path(path) = &*field.base {
                if path.path.get_ident().is_some() {
                    if let syn::Member::Named(name) = &field.member {
                        return Some(syn::parse_quote!(#name));
                    }
                }
            }
            // Otherwise recurse
            if let Some(new_base) = strip_instruction_arg_prefix(&field.base) {
                let member = &field.member;
                return Some(syn::parse_quote!(#new_base.#member));
            }
            None
        }
        Expr::MethodCall(mc) => {
            if let Some(new_receiver) = strip_instruction_arg_prefix(&mc.receiver) {
                let method = &mc.method;
                let args = &mc.args;
                let turbofish = &mc.turbofish;
                return Some(if let Some(tf) = turbofish {
                    syn::parse_quote!(#new_receiver.#method #tf (#args))
                } else {
                    syn::parse_quote!(#new_receiver.#method(#args))
                });
            }
            None
        }
        Expr::Index(idx) => {
            if let Some(new_expr) = strip_instruction_arg_prefix(&idx.expr) {
                let index = &idx.index;
                return Some(syn::parse_quote!(#new_expr[#index]));
            }
            None
        }
        Expr::Reference(r) => {
            if let Some(stripped) = strip_instruction_arg_prefix(&r.expr) {
                return Some(if r.mutability.is_some() {
                    syn::parse_quote!(&mut #stripped)
                } else {
                    syn::parse_quote!(&#stripped)
                });
            }
            None
        }
        _ => None,
    }
}

/// Extract terminal data field from instruction arg expression.
///
/// For `params.owner.as_ref()`, returns `owner`.
/// For `params.id.to_le_bytes()`, returns `id`.
fn extract_terminal_data_field(expr: &Expr, root_name: &str) -> Option<Ident> {
    match expr {
        Expr::Field(field) => {
            // Check if base is the root instruction arg
            if let Expr::Path(path) = &*field.base {
                if let Some(ident) = path.path.get_ident() {
                    if ident == root_name {
                        if let syn::Member::Named(name) = &field.member {
                            return Some(name.clone());
                        }
                    }
                }
            }
            // Otherwise recurse
            extract_terminal_data_field(&field.base, root_name)
        }
        Expr::MethodCall(mc) => extract_terminal_data_field(&mc.receiver, root_name),
        Expr::Index(idx) => extract_terminal_data_field(&idx.expr, root_name),
        Expr::Reference(r) => extract_terminal_data_field(&r.expr, root_name),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    fn make_instruction_args(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn make_account_fields(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    // =========================================================================
    // Literal tests
    // =========================================================================

    #[test]
    fn test_literal_bare() {
        let expr: Expr = parse_quote!(b"record");
        let result =
            classify_seed(&expr, &HashSet::new(), &HashSet::new()).expect("should classify");

        assert_eq!(result.kind, SeedKind::Constant);
        assert!(result.field.is_none());
    }

    #[test]
    fn test_literal_string() {
        let expr: Expr = parse_quote!("record");
        let result =
            classify_seed(&expr, &HashSet::new(), &HashSet::new()).expect("should classify");

        assert_eq!(result.kind, SeedKind::Constant);
        assert!(result.field.is_none());
    }

    #[test]
    fn test_literal_sliced() {
        let expr: Expr = parse_quote!(b"literal"[..]);
        let result =
            classify_seed(&expr, &HashSet::new(), &HashSet::new()).expect("should classify");

        assert_eq!(result.kind, SeedKind::Constant);
        assert!(result.field.is_none());
    }

    #[test]
    fn test_literal_referenced() {
        let expr: Expr = parse_quote!(&b"seed");
        let result =
            classify_seed(&expr, &HashSet::new(), &HashSet::new()).expect("should classify");

        assert_eq!(result.kind, SeedKind::Constant);
        assert!(result.field.is_none());
    }

    // =========================================================================
    // Constant tests
    // =========================================================================

    #[test]
    fn test_constant_bare() {
        let expr: Expr = parse_quote!(SEED_CONSTANT);
        let result =
            classify_seed(&expr, &HashSet::new(), &HashSet::new()).expect("should classify");

        assert_eq!(result.kind, SeedKind::Constant);
        assert!(result.field.is_none());
    }

    #[test]
    fn test_constant_qualified() {
        let expr: Expr = parse_quote!(crate::state::SEED_CONSTANT);
        let result =
            classify_seed(&expr, &HashSet::new(), &HashSet::new()).expect("should classify");

        assert_eq!(result.kind, SeedKind::Constant);
        assert!(result.field.is_none());
    }

    // =========================================================================
    // Account tests
    // =========================================================================

    #[test]
    fn test_ctx_account_bare() {
        let expr: Expr = parse_quote!(authority.key().as_ref());
        let account_fields = make_account_fields(&["authority"]);

        let result =
            classify_seed(&expr, &HashSet::new(), &account_fields).expect("should classify");

        assert_eq!(result.kind, SeedKind::Account);
        assert_eq!(result.field.unwrap().to_string(), "authority");
    }

    #[test]
    fn test_ctx_account_nested() {
        let expr: Expr = parse_quote!(ctx.accounts.authority.key().as_ref());
        let account_fields = make_account_fields(&["authority"]);

        let result =
            classify_seed(&expr, &HashSet::new(), &account_fields).expect("should classify");

        assert_eq!(result.kind, SeedKind::Account);
        assert_eq!(result.field.unwrap().to_string(), "authority");
    }

    // =========================================================================
    // Data tests
    // =========================================================================

    #[test]
    fn test_data_field_bare() {
        let expr: Expr = parse_quote!(owner.as_ref());
        let instruction_args = make_instruction_args(&["owner"]);

        let result =
            classify_seed(&expr, &instruction_args, &HashSet::new()).expect("should classify");

        assert_eq!(result.kind, SeedKind::Data);
        assert_eq!(result.field.unwrap().to_string(), "owner");
    }

    #[test]
    fn test_data_field_struct() {
        let expr: Expr = parse_quote!(params.owner.as_ref());
        let instruction_args = make_instruction_args(&["params"]);

        let result =
            classify_seed(&expr, &instruction_args, &HashSet::new()).expect("should classify");

        assert_eq!(result.kind, SeedKind::Data);
        assert_eq!(result.field.unwrap().to_string(), "owner");
    }

    #[test]
    fn test_data_field_bytes() {
        let expr: Expr = parse_quote!(params.id.to_le_bytes().as_ref());
        let instruction_args = make_instruction_args(&["params"]);

        let result =
            classify_seed(&expr, &instruction_args, &HashSet::new()).expect("should classify");

        assert_eq!(result.kind, SeedKind::Data);
        assert_eq!(result.field.unwrap().to_string(), "id");
    }

    // =========================================================================
    // Passthrough tests (complex expressions -> Constant)
    // =========================================================================

    #[test]
    fn test_passthrough_fn() {
        let expr: Expr = parse_quote!(identity_seed::<12>(b"seed"));
        let result =
            classify_seed(&expr, &HashSet::new(), &HashSet::new()).expect("should classify");

        assert_eq!(result.kind, SeedKind::Constant);
        assert!(result.field.is_none());
    }

    #[test]
    fn test_passthrough_trait() {
        let expr: Expr = parse_quote!(<Type as Trait>::SEED);
        let result =
            classify_seed(&expr, &HashSet::new(), &HashSet::new()).expect("should classify");

        assert_eq!(result.kind, SeedKind::Constant);
        assert!(result.field.is_none());
    }

    // =========================================================================
    // Mixed/edge case tests
    // =========================================================================

    #[test]
    fn test_account_vs_instruction_arg_precedence() {
        // When a name is in both sets, instruction_args should take precedence
        let expr: Expr = parse_quote!(owner.as_ref());
        let instruction_args = make_instruction_args(&["owner"]);
        let account_fields = make_account_fields(&["owner"]);

        let result =
            classify_seed(&expr, &instruction_args, &account_fields).expect("should classify");

        // Instruction args checked first
        assert_eq!(result.kind, SeedKind::Data);
    }

    #[test]
    fn test_unknown_identifier_becomes_constant() {
        // An unknown identifier (not in either set) becomes Constant
        let expr: Expr = parse_quote!(unknown_thing.as_ref());
        let result =
            classify_seed(&expr, &HashSet::new(), &HashSet::new()).expect("should classify");

        assert_eq!(result.kind, SeedKind::Constant);
    }

    // =========================================================================
    // Helper function tests
    // =========================================================================

    #[test]
    fn test_is_literal() {
        assert!(is_literal(&parse_quote!(b"test")));
        assert!(is_literal(&parse_quote!("test")));
        assert!(is_literal(&parse_quote!(b"test"[..])));
        assert!(is_literal(&parse_quote!(&b"test")));
        assert!(!is_literal(&parse_quote!(CONSTANT)));
        assert!(!is_literal(&parse_quote!(field.as_ref())));
    }

    #[test]
    fn test_is_constant_path() {
        assert!(is_constant_path(&parse_quote!(SEED)));
        assert!(is_constant_path(&parse_quote!(SEED_PREFIX)));
        assert!(is_constant_path(&parse_quote!(crate::SEED)));
        assert!(is_constant_path(&parse_quote!(crate::state::SEED_CONSTANT)));
        assert!(!is_constant_path(&parse_quote!(seed)));
        assert!(!is_constant_path(&parse_quote!(MyConstant)));
        assert!(!is_constant_path(&parse_quote!(<T as Trait>::SEED))); // type-qualified
    }
}
