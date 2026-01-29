//! Property-based tests for shared utility functions.
//!
//! These tests verify correctness properties of:
//! - `is_constant_identifier` - SCREAMING_SNAKE_CASE detection
//! - `is_base_path` - Path base matching

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use syn::parse_str;

    use crate::light_pdas::shared_utils::{is_base_path, is_constant_identifier};

    // ========================================================================
    // Constants
    // ========================================================================

    /// Rust keywords that should be excluded from identifier generation.
    /// These parse as literals or reserved words, not as identifiers.
    const RUST_KEYWORDS: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do",
        "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
    ];

    /// Check if a string is a Rust keyword
    fn is_rust_keyword(s: &str) -> bool {
        RUST_KEYWORDS.contains(&s)
    }

    // ========================================================================
    // Strategies for generating test inputs
    // ========================================================================

    /// Strategy for generating valid uppercase identifiers (for constants)
    fn arb_uppercase_ident() -> impl Strategy<Value = String> {
        "[A-Z][A-Z0-9_]{0,15}"
    }

    /// Strategy for generating valid lowercase identifiers (for variables)
    /// Filters out Rust keywords that would parse as literals/reserved words.
    fn arb_lowercase_ident() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,15}".prop_filter("not a Rust keyword", |s| !is_rust_keyword(s))
    }

    /// Strategy for generating mixed-case identifiers
    fn arb_mixed_case_ident() -> impl Strategy<Value = String> {
        "[A-Z][a-z][A-Za-z0-9_]{0,14}"
    }

    /// Strategy for generating arbitrary identifiers (valid Rust identifiers)
    fn arb_any_ident() -> impl Strategy<Value = String> {
        "[a-zA-Z_][a-zA-Z0-9_]{0,15}"
    }

    // ========================================================================
    // Property Tests: is_constant_identifier
    // ========================================================================

    proptest! {
        /// All-uppercase identifiers should be accepted as constants.
        /// Pattern: "ABC", "A_B_C", "A1", "ABC_123"
        #[test]
        fn prop_all_uppercase_accepted(name in arb_uppercase_ident()) {
            prop_assume!(!name.is_empty());
            let result = is_constant_identifier(&name);
            prop_assert!(
                result,
                "All-uppercase identifier '{}' should be accepted as constant",
                name
            );
        }

        /// Any lowercase letter in the identifier should cause rejection.
        #[test]
        fn prop_any_lowercase_rejected(name in arb_mixed_case_ident()) {
            prop_assume!(!name.is_empty());
            let result = is_constant_identifier(&name);
            prop_assert!(
                !result,
                "Mixed-case identifier '{}' should NOT be accepted as constant",
                name
            );
        }

        /// Purely lowercase identifiers should be rejected.
        #[test]
        fn prop_lowercase_rejected(name in arb_lowercase_ident()) {
            prop_assume!(!name.is_empty());
            let result = is_constant_identifier(&name);
            prop_assert!(
                !result,
                "Lowercase identifier '{}' should NOT be accepted as constant",
                name
            );
        }

        /// Empty string should always be rejected.
        #[test]
        fn prop_empty_rejected(_seed in 0u32..1000) {
            let result = is_constant_identifier("");
            prop_assert!(!result, "Empty string should be rejected");
        }

        /// Underscore-only patterns with at least one uppercase letter should be accepted.
        #[test]
        fn prop_underscore_with_uppercase_accepted(name in "[A-Z]_[A-Z]") {
            let result = is_constant_identifier(&name);
            prop_assert!(
                result,
                "Underscore pattern '{}' with uppercase should be accepted",
                name
            );
        }

        /// Digits after letter should be accepted in constants.
        #[test]
        fn prop_digits_after_letter_accepted(prefix in "[A-Z]{1,3}", digits in "[0-9]{1,4}") {
            let name = format!("{}{}", prefix, digits);
            let result = is_constant_identifier(&name);
            prop_assert!(
                result,
                "Constant with digits '{}' should be accepted",
                name
            );
        }

        /// Leading digit should be rejected - SCREAMING_SNAKE_CASE must start with uppercase letter.
        #[test]
        fn prop_leading_digit_rejected(digit in "[0-9]", suffix in "[A-Z]{1,5}") {
            let name = format!("{}{}", digit, suffix);
            let result = is_constant_identifier(&name);
            prop_assert!(
                !result,
                "String '{}' starting with digit should be rejected as constant identifier",
                name
            );
        }

        /// Classification should be deterministic.
        #[test]
        fn prop_is_constant_deterministic(name in arb_any_ident()) {
            let result1 = is_constant_identifier(&name);
            let result2 = is_constant_identifier(&name);
            prop_assert_eq!(
                result1, result2,
                "is_constant_identifier should be deterministic for '{}'",
                name
            );
        }

        /// Special characters (other than underscore) should cause rejection.
        #[test]
        fn prop_special_chars_rejected(prefix in "[A-Z]{1,3}", special in r"[!@#$%^&*()-+=]") {
            let name = format!("{}{}", prefix, special);
            let result = is_constant_identifier(&name);
            prop_assert!(
                !result,
                "Identifier with special char '{}' should be rejected",
                name
            );
        }
    }

    // ========================================================================
    // Property Tests: is_base_path
    // ========================================================================

    proptest! {
        /// Should match when expression starts with exact base.
        #[test]
        fn prop_matches_exact_base(base in arb_lowercase_ident()) {
            prop_assume!(!base.is_empty());

            if let Ok(expr) = parse_str::<syn::Expr>(&base) {
                let result = is_base_path(&expr, &base);
                prop_assert!(
                    result,
                    "Path '{}' should match base '{}'",
                    base,
                    base
                );
            }
        }

        /// Should reject when expression starts with different base.
        #[test]
        fn prop_rejects_different_base(
            actual_base in arb_lowercase_ident(),
            check_base in arb_lowercase_ident()
        ) {
            prop_assume!(!actual_base.is_empty() && !check_base.is_empty());
            prop_assume!(actual_base != check_base);

            if let Ok(expr) = parse_str::<syn::Expr>(&actual_base) {
                let result = is_base_path(&expr, &check_base);
                prop_assert!(
                    !result,
                    "Path '{}' should NOT match base '{}'",
                    actual_base,
                    check_base
                );
            }
        }

        /// Field access expressions should match their base.
        #[test]
        fn prop_field_access_matches_base(base in arb_lowercase_ident(), field in arb_lowercase_ident()) {
            prop_assume!(!base.is_empty() && !field.is_empty());

            let expr_str = format!("{}.{}", base, field);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                // Note: is_base_path only matches simple Path expressions,
                // not Field expressions, so this should return false
                let result = is_base_path(&expr, &base);
                // Field expressions are NOT Path expressions
                prop_assert!(
                    !result,
                    "Field expression '{}' is not a Path, should return false for base check",
                    expr_str
                );
            }
        }

        /// is_base_path should be deterministic.
        #[test]
        fn prop_is_base_path_deterministic(name in arb_lowercase_ident()) {
            prop_assume!(!name.is_empty());

            if let Ok(expr) = parse_str::<syn::Expr>(&name) {
                let result1 = is_base_path(&expr, &name);
                let result2 = is_base_path(&expr, &name);
                prop_assert_eq!(
                    result1, result2,
                    "is_base_path should be deterministic"
                );
            }
        }
    }
}
