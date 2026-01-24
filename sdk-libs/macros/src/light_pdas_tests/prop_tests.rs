//! Property-based tests for seed parsing correctness.
//!
//! These tests verify that `classify_seed_expr` correctly classifies seed expressions
//! and preserves their semantic content. Unlike fuzz tests that only verify crash-freedom,
//! property tests verify correctness properties like:
//! - Literal bytes are preserved exactly
//! - Uppercase identifiers are classified as constants
//! - Instruction args are correctly distinguished from ctx accounts
//! - Classification is deterministic

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use syn::parse_str;

    use crate::light_pdas::{
        account::seed_extraction::{classify_seed_expr, ClassifiedSeed, InstructionArgSet},
        shared_utils::is_constant_identifier,
    };

    // ========================================================================
    // Helper functions
    // ========================================================================

    /// Check if ClassifiedSeed variants match (ignoring inner values)
    fn variant_matches(a: &ClassifiedSeed, b: &ClassifiedSeed) -> bool {
        matches!(
            (a, b),
            (ClassifiedSeed::Literal(_), ClassifiedSeed::Literal(_))
                | (ClassifiedSeed::Constant(_), ClassifiedSeed::Constant(_))
                | (ClassifiedSeed::CtxAccount(_), ClassifiedSeed::CtxAccount(_))
                | (
                    ClassifiedSeed::DataField { .. },
                    ClassifiedSeed::DataField { .. }
                )
                | (
                    ClassifiedSeed::FunctionCall { .. },
                    ClassifiedSeed::FunctionCall { .. }
                )
        )
    }

    // ========================================================================
    // Strategies for generating test inputs
    // ========================================================================

    /// Strategy for generating valid lowercase identifiers (for accounts/fields)
    fn arb_lowercase_ident() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,15}"
    }

    /// Strategy for generating valid uppercase identifiers (for constants)
    fn arb_uppercase_ident() -> impl Strategy<Value = String> {
        "[A-Z][A-Z0-9_]{0,15}"
    }

    /// Strategy for generating ASCII bytes safe for byte string literals
    /// Excludes quotes, backslashes, and non-printable characters
    fn arb_safe_ascii_bytes() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(
            prop::sample::select(
                (0x20u8..=0x7E)
                    .filter(|&b| b != b'"' && b != b'\\')
                    .collect::<Vec<_>>(),
            ),
            0..32,
        )
    }

    /// Strategy for generating valid seed expression strings
    fn arb_seed_expr() -> impl Strategy<Value = String> {
        prop_oneof![
            // Literals (use safe ASCII for reliability)
            "[a-z]{1,20}".prop_map(|s| format!("b\"{}\"", s)),
            "[a-z]{1,20}".prop_map(|s| format!("b\"{}\"[..]", s)),
            // Constants
            arb_uppercase_ident(),
            // Account refs
            arb_lowercase_ident().prop_map(|s| format!("{}.key().as_ref()", s)),
            // Data fields
            arb_lowercase_ident().prop_map(|s| format!("params.{}.as_ref()", s)),
            // With conversion
            arb_lowercase_ident().prop_map(|s| format!("params.{}.to_le_bytes().as_ref()", s)),
        ]
    }

    // ========================================================================
    // Property 1: Literal Byte Preservation
    // ========================================================================

    proptest! {
        /// Byte string literals should preserve their bytes exactly.
        #[test]
        fn literal_preserves_bytes(bytes in arb_safe_ascii_bytes()) {
            // Convert bytes to string for byte literal
            let byte_str = String::from_utf8(bytes.clone()).unwrap_or_default();
            if byte_str.is_empty() {
                return Ok(());
            }

            let literal = format!("b\"{}\"", byte_str);
            if let Ok(expr) = parse_str::<syn::Expr>(&literal) {
                let args = InstructionArgSet::empty();
                let result = classify_seed_expr(&expr, &args);

                if let Ok(ClassifiedSeed::Literal(output_bytes)) = result {
                    prop_assert_eq!(output_bytes, bytes, "Literal bytes should be preserved exactly");
                }
            }
        }

        /// Byte string literals with slice syntax should also preserve bytes.
        #[test]
        fn literal_slice_preserves_bytes(s in "[a-z]{1,20}") {
            let literal = format!("b\"{}\"[..]", s);
            let expr: syn::Expr = parse_str(&literal).unwrap();
            let args = InstructionArgSet::empty();
            let result = classify_seed_expr(&expr, &args).unwrap();

            if let ClassifiedSeed::Literal(output_bytes) = result {
                prop_assert_eq!(output_bytes, s.as_bytes(), "Slice literal bytes should be preserved");
            } else {
                prop_assert!(false, "Expected Literal variant");
            }
        }
    }

    // ========================================================================
    // Property 2: Constant Detection (Uppercase Rule)
    // ========================================================================

    proptest! {
        /// All-uppercase identifiers should be classified as constants.
        #[test]
        fn uppercase_is_constant(name in arb_uppercase_ident()) {
            prop_assume!(!name.is_empty());
            prop_assume!(is_constant_identifier(&name));

            if let Ok(expr) = parse_str::<syn::Expr>(&name) {
                let args = InstructionArgSet::empty();
                let result = classify_seed_expr(&expr, &args);

                if let Ok(classified) = result {
                    prop_assert!(
                        matches!(classified, ClassifiedSeed::Constant(_)),
                        "Uppercase identifier '{}' should be Constant, got {:?}",
                        name,
                        classified
                    );
                }
            }
        }

        /// Lowercase identifiers should NOT be classified as constants.
        #[test]
        fn lowercase_not_constant(name in arb_lowercase_ident()) {
            prop_assume!(!name.is_empty());
            prop_assume!(!is_constant_identifier(&name));

            if let Ok(expr) = parse_str::<syn::Expr>(&name) {
                let args = InstructionArgSet::empty();
                let result = classify_seed_expr(&expr, &args);

                if let Ok(classified) = result {
                    prop_assert!(
                        !matches!(classified, ClassifiedSeed::Constant(_)),
                        "Lowercase identifier '{}' should NOT be Constant, got {:?}",
                        name,
                        classified
                    );
                }
            }
        }

        /// Mixed-case identifiers starting with uppercase should NOT be constants.
        #[test]
        fn mixed_case_not_constant(upper in "[A-Z]", lower in "[a-z]{1,10}") {
            let name = format!("{}{}", upper, lower);
            prop_assume!(!is_constant_identifier(&name));

            if let Ok(expr) = parse_str::<syn::Expr>(&name) {
                let args = InstructionArgSet::empty();
                let result = classify_seed_expr(&expr, &args);

                if let Ok(classified) = result {
                    prop_assert!(
                        !matches!(classified, ClassifiedSeed::Constant(_)),
                        "Mixed-case identifier '{}' should NOT be Constant",
                        name
                    );
                }
            }
        }
    }

    // ========================================================================
    // Property 3: Instruction Arg Detection
    // ========================================================================

    proptest! {
        /// An identifier that IS in instruction_args should become DataField.
        #[test]
        fn instruction_arg_becomes_data_field(name in arb_lowercase_ident()) {
            prop_assume!(!name.is_empty());
            prop_assume!(!is_constant_identifier(&name));

            if let Ok(expr) = parse_str::<syn::Expr>(&name) {
                let args = InstructionArgSet::from_names(vec![name.clone()]);
                let result = classify_seed_expr(&expr, &args);

                if let Ok(classified) = result {
                    prop_assert!(
                        matches!(classified, ClassifiedSeed::DataField { .. }),
                        "Identifier '{}' in instruction_args should be DataField, got {:?}",
                        name,
                        classified
                    );
                }
            }
        }

        /// An identifier that is NOT in instruction_args should become CtxAccount.
        #[test]
        fn non_instruction_arg_becomes_ctx_account(name in arb_lowercase_ident()) {
            prop_assume!(!name.is_empty());
            prop_assume!(!is_constant_identifier(&name));

            if let Ok(expr) = parse_str::<syn::Expr>(&name) {
                let args = InstructionArgSet::empty(); // name NOT in args
                let result = classify_seed_expr(&expr, &args);

                if let Ok(classified) = result {
                    prop_assert!(
                        matches!(classified, ClassifiedSeed::CtxAccount(_)),
                        "Identifier '{}' NOT in instruction_args should be CtxAccount, got {:?}",
                        name,
                        classified
                    );
                }
            }
        }

        /// Field access on instruction arg should extract the field name.
        #[test]
        fn instruction_arg_field_access(
            param_name in arb_lowercase_ident(),
            field_name in arb_lowercase_ident()
        ) {
            prop_assume!(!param_name.is_empty() && !field_name.is_empty());
            prop_assume!(param_name != field_name); // Avoid ambiguity

            let expr_str = format!("{}.{}", param_name, field_name);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::from_names(vec![param_name.clone()]);
                let result = classify_seed_expr(&expr, &args);

                if let Ok(ClassifiedSeed::DataField { field_name: extracted, .. }) = result {
                    prop_assert_eq!(
                        extracted.to_string(),
                        field_name,
                        "Field name should be extracted correctly"
                    );
                }
            }
        }
    }

    // ========================================================================
    // Property 4: Method Call Unwrapping (.as_ref() transparency)
    // ========================================================================

    proptest! {
        /// .as_ref() should be transparent - the underlying expression type is preserved.
        #[test]
        fn as_ref_preserves_base_classification(name in arb_lowercase_ident()) {
            prop_assume!(!name.is_empty());
            prop_assume!(!is_constant_identifier(&name));

            let base_expr_str = name.clone();
            let with_as_ref = format!("{}.as_ref()", name);

            if let (Ok(base_expr), Ok(wrapped_expr)) = (
                parse_str::<syn::Expr>(&base_expr_str),
                parse_str::<syn::Expr>(&with_as_ref)
            ) {
                let args = InstructionArgSet::empty();
                let base_result = classify_seed_expr(&base_expr, &args);
                let wrapped_result = classify_seed_expr(&wrapped_expr, &args);

                // Both should succeed or both should fail
                prop_assert_eq!(
                    base_result.is_ok(),
                    wrapped_result.is_ok(),
                    "Base and wrapped should have same success/failure"
                );

                if let (Ok(base), Ok(wrapped)) = (base_result, wrapped_result) {
                    prop_assert!(
                        variant_matches(&base, &wrapped),
                        "as_ref() should be transparent: base={:?}, wrapped={:?}",
                        base,
                        wrapped
                    );
                }
            }
        }

        /// .as_bytes() should also be transparent.
        #[test]
        fn as_bytes_preserves_base_classification(name in arb_lowercase_ident()) {
            prop_assume!(!name.is_empty());
            prop_assume!(!is_constant_identifier(&name));

            let base_expr_str = name.clone();
            let with_as_bytes = format!("{}.as_bytes()", name);

            if let (Ok(base_expr), Ok(wrapped_expr)) = (
                parse_str::<syn::Expr>(&base_expr_str),
                parse_str::<syn::Expr>(&with_as_bytes)
            ) {
                let args = InstructionArgSet::empty();
                let base_result = classify_seed_expr(&base_expr, &args);
                let wrapped_result = classify_seed_expr(&wrapped_expr, &args);

                prop_assert_eq!(
                    base_result.is_ok(),
                    wrapped_result.is_ok(),
                    "Base and wrapped should have same success/failure"
                );

                if let (Ok(base), Ok(wrapped)) = (base_result, wrapped_result) {
                    prop_assert!(
                        variant_matches(&base, &wrapped),
                        "as_bytes() should be transparent"
                    );
                }
            }
        }
    }

    // ========================================================================
    // Property 5: Determinism
    // ========================================================================

    proptest! {
        /// Classification should be deterministic - same input always gives same output.
        #[test]
        fn classification_is_deterministic(expr_str in arb_seed_expr()) {
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::from_names(vec!["params".to_string()]);

                let result1 = classify_seed_expr(&expr, &args);
                let result2 = classify_seed_expr(&expr, &args);

                prop_assert_eq!(
                    result1.is_ok(),
                    result2.is_ok(),
                    "Classification should consistently succeed or fail"
                );

                if let (Ok(r1), Ok(r2)) = (result1, result2) {
                    prop_assert_eq!(
                        format!("{:?}", r1),
                        format!("{:?}", r2),
                        "Classification should be deterministic"
                    );
                }
            }
        }

        /// Classification with different args should still be deterministic per args.
        #[test]
        fn classification_deterministic_with_varying_args(
            name in arb_lowercase_ident(),
            include_in_args in prop::bool::ANY
        ) {
            prop_assume!(!name.is_empty());
            prop_assume!(!is_constant_identifier(&name));

            if let Ok(expr) = parse_str::<syn::Expr>(&name) {
                let args = if include_in_args {
                    InstructionArgSet::from_names(vec![name.clone()])
                } else {
                    InstructionArgSet::empty()
                };

                let result1 = classify_seed_expr(&expr, &args);
                let result2 = classify_seed_expr(&expr, &args);

                prop_assert_eq!(
                    format!("{:?}", result1),
                    format!("{:?}", result2),
                    "Classification should be deterministic with same args"
                );
            }
        }
    }

    // ========================================================================
    // Property 6: Field Name Extraction
    // ========================================================================

    proptest! {
        /// params.field_name.as_ref() should extract the correct field name.
        #[test]
        fn extracts_correct_field_name(field in arb_lowercase_ident()) {
            prop_assume!(!field.is_empty());

            let expr_str = format!("params.{}.as_ref()", field);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::from_names(vec!["params".to_string()]);
                let result = classify_seed_expr(&expr, &args);

                if let Ok(ClassifiedSeed::DataField { field_name, .. }) = result {
                    prop_assert_eq!(
                        field_name.to_string(),
                        field,
                        "Field name should be extracted correctly"
                    );
                } else {
                    prop_assert!(false, "Expected DataField variant for params.{}.as_ref()", field);
                }
            }
        }

        /// Nested field access should extract the terminal field name.
        #[test]
        fn extracts_terminal_field_from_nested(
            middle in arb_lowercase_ident(),
            terminal in arb_lowercase_ident()
        ) {
            prop_assume!(!middle.is_empty() && !terminal.is_empty());
            prop_assume!(middle != terminal);

            let expr_str = format!("params.{}.{}.as_ref()", middle, terminal);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::from_names(vec!["params".to_string()]);
                let result = classify_seed_expr(&expr, &args);

                if let Ok(ClassifiedSeed::DataField { field_name, .. }) = result {
                    prop_assert_eq!(
                        field_name.to_string(),
                        terminal,
                        "Terminal field name should be extracted from nested access"
                    );
                }
            }
        }
    }

    // ========================================================================
    // Property 7: Conversion Method Capture
    // ========================================================================

    proptest! {
        /// to_le_bytes() conversion should be captured in the result.
        #[test]
        fn captures_to_le_bytes_conversion(field in arb_lowercase_ident()) {
            prop_assume!(!field.is_empty());

            let expr_str = format!("params.{}.to_le_bytes().as_ref()", field);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::from_names(vec!["params".to_string()]);
                let result = classify_seed_expr(&expr, &args);

                if let Ok(ClassifiedSeed::DataField { conversion, field_name, .. }) = result {
                    prop_assert_eq!(
                        field_name.to_string(),
                        field,
                        "Field name should match"
                    );
                    prop_assert!(
                        conversion.is_some(),
                        "Conversion should be captured"
                    );
                    prop_assert_eq!(
                        conversion.map(|c| c.to_string()),
                        Some("to_le_bytes".to_string()),
                        "Conversion should be to_le_bytes"
                    );
                } else {
                    prop_assert!(false, "Expected DataField variant");
                }
            }
        }

        /// to_be_bytes() conversion should also be captured.
        #[test]
        fn captures_to_be_bytes_conversion(field in arb_lowercase_ident()) {
            prop_assume!(!field.is_empty());

            let expr_str = format!("params.{}.to_be_bytes().as_ref()", field);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::from_names(vec!["params".to_string()]);
                let result = classify_seed_expr(&expr, &args);

                if let Ok(ClassifiedSeed::DataField { conversion, .. }) = result {
                    prop_assert_eq!(
                        conversion.map(|c| c.to_string()),
                        Some("to_be_bytes".to_string()),
                        "Conversion should be to_be_bytes"
                    );
                }
            }
        }

        /// Bare instruction arg with to_le_bytes should capture conversion.
        #[test]
        fn bare_arg_captures_conversion(arg_name in arb_lowercase_ident()) {
            prop_assume!(!arg_name.is_empty());
            prop_assume!(!is_constant_identifier(&arg_name));

            let expr_str = format!("{}.to_le_bytes().as_ref()", arg_name);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::from_names(vec![arg_name.clone()]);
                let result = classify_seed_expr(&expr, &args);

                if let Ok(ClassifiedSeed::DataField { field_name, conversion }) = result {
                    prop_assert_eq!(
                        field_name.to_string(),
                        arg_name,
                        "Field name should be the arg name itself"
                    );
                    prop_assert_eq!(
                        conversion.map(|c| c.to_string()),
                        Some("to_le_bytes".to_string()),
                        "Conversion should be captured"
                    );
                }
            }
        }
    }

    // ========================================================================
    // Property 8: Account Key Method
    // ========================================================================

    proptest! {
        /// account.key().as_ref() should be classified as CtxAccount.
        #[test]
        fn account_key_as_ref_is_ctx_account(account in arb_lowercase_ident()) {
            prop_assume!(!account.is_empty());
            prop_assume!(!is_constant_identifier(&account));

            let expr_str = format!("{}.key().as_ref()", account);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::empty();
                let result = classify_seed_expr(&expr, &args);

                if let Ok(classified) = result {
                    prop_assert!(
                        matches!(classified, ClassifiedSeed::CtxAccount(ref ident) if *ident == account),
                        "account.key().as_ref() should be CtxAccount({}), got {:?}",
                        account,
                        classified
                    );
                }
            }
        }

        /// Account key on instruction arg field should be DataField.
        #[test]
        fn instruction_arg_key_is_data_field(
            param in arb_lowercase_ident(),
            field in arb_lowercase_ident()
        ) {
            prop_assume!(!param.is_empty() && !field.is_empty());

            let expr_str = format!("{}.{}.key().as_ref()", param, field);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::from_names(vec![param.clone()]);
                let result = classify_seed_expr(&expr, &args);

                if let Ok(ClassifiedSeed::DataField { field_name, .. }) = result {
                    prop_assert_eq!(
                        field_name.to_string(),
                        field,
                        "Field name should be extracted from key() call on instruction arg"
                    );
                }
            }
        }
    }

    // ========================================================================
    // Property 9: Reference Unwrapping
    // ========================================================================

    proptest! {
        /// &expr should be equivalent to expr for classification purposes.
        #[test]
        fn reference_is_transparent(name in arb_lowercase_ident()) {
            prop_assume!(!name.is_empty());
            prop_assume!(!is_constant_identifier(&name));

            let base_str = name.clone();
            let ref_str = format!("&{}", name);

            if let (Ok(base_expr), Ok(ref_expr)) = (
                parse_str::<syn::Expr>(&base_str),
                parse_str::<syn::Expr>(&ref_str)
            ) {
                let args = InstructionArgSet::empty();
                let base_result = classify_seed_expr(&base_expr, &args);
                let ref_result = classify_seed_expr(&ref_expr, &args);

                prop_assert_eq!(
                    base_result.is_ok(),
                    ref_result.is_ok(),
                    "Base and ref should have same success/failure"
                );

                if let (Ok(base), Ok(referenced)) = (base_result, ref_result) {
                    prop_assert!(
                        variant_matches(&base, &referenced),
                        "& should be transparent: base={:?}, ref={:?}",
                        base,
                        referenced
                    );
                }
            }
        }
    }

    // ========================================================================
    // Property 10: Instruction Arg Precedence
    // ========================================================================

    proptest! {
        /// When a name is in instruction_args, it should always be DataField, not CtxAccount.
        /// This tests the precedence rule.
        #[test]
        fn instruction_arg_takes_precedence(name in arb_lowercase_ident()) {
            prop_assume!(!name.is_empty());
            prop_assume!(!is_constant_identifier(&name));

            if let Ok(expr) = parse_str::<syn::Expr>(&name) {
                // With empty args -> CtxAccount
                let empty_args = InstructionArgSet::empty();
                let without_arg = classify_seed_expr(&expr, &empty_args);

                // With name in args -> DataField
                let with_args = InstructionArgSet::from_names(vec![name.clone()]);
                let with_arg = classify_seed_expr(&expr, &with_args);

                if let (Ok(without), Ok(with)) = (without_arg, with_arg) {
                    prop_assert!(
                        matches!(without, ClassifiedSeed::CtxAccount(_)),
                        "Without args, '{}' should be CtxAccount",
                        name
                    );
                    prop_assert!(
                        matches!(with, ClassifiedSeed::DataField { .. }),
                        "With args, '{}' should be DataField",
                        name
                    );
                }
            }
        }
    }

    // ========================================================================
    // Property 11: Rust Keywords in Seeds
    // ========================================================================

    proptest! {
        /// `self.field` expressions should be classified as CtxAccount.
        /// The `self` keyword refers to the struct, and field access on it
        /// should be treated as a context account reference.
        #[test]
        fn self_field_is_ctx_account(field in arb_lowercase_ident()) {
            prop_assume!(!field.is_empty());

            let expr_str = format!("self.{}", field);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::empty();
                let result = classify_seed_expr(&expr, &args);

                prop_assert!(
                    result.is_ok(),
                    "self.{} should parse successfully",
                    field
                );
                prop_assert!(
                    matches!(result.unwrap(), ClassifiedSeed::CtxAccount(_)),
                    "self.{} should be classified as CtxAccount",
                    field
                );
            }
        }

        /// `self.field.as_ref()` should be classified as CtxAccount.
        #[test]
        fn self_field_as_ref_is_ctx_account(field in arb_lowercase_ident()) {
            prop_assume!(!field.is_empty());

            let expr_str = format!("self.{}.as_ref()", field);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::empty();
                let result = classify_seed_expr(&expr, &args);

                prop_assert!(
                    result.is_ok(),
                    "self.{}.as_ref() should parse successfully",
                    field
                );
                prop_assert!(
                    matches!(result.unwrap(), ClassifiedSeed::CtxAccount(_)),
                    "self.{}.as_ref() should be classified as CtxAccount",
                    field
                );
            }
        }

        /// `self.field.key()` should be classified as CtxAccount.
        #[test]
        fn self_field_key_is_ctx_account(field in arb_lowercase_ident()) {
            prop_assume!(!field.is_empty());

            let expr_str = format!("self.{}.key()", field);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::empty();
                let result = classify_seed_expr(&expr, &args);

                prop_assert!(
                    result.is_ok(),
                    "self.{}.key() should parse successfully",
                    field
                );
                prop_assert!(
                    matches!(result.unwrap(), ClassifiedSeed::CtxAccount(_)),
                    "self.{}.key() should be classified as CtxAccount",
                    field
                );
            }
        }

        /// `Self::CONSTANT` should be classified as Constant.
        #[test]
        fn self_type_constant_is_constant(constant in arb_uppercase_ident()) {
            prop_assume!(!constant.is_empty());

            let expr_str = format!("Self::{}", constant);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::empty();
                let result = classify_seed_expr(&expr, &args);

                prop_assert!(
                    result.is_ok(),
                    "Self::{} should parse successfully",
                    constant
                );
                prop_assert!(
                    matches!(result.unwrap(), ClassifiedSeed::Constant(_)),
                    "Self::{} should be classified as Constant",
                    constant
                );
            }
        }

        /// `crate::CONSTANT` should be classified as Constant.
        #[test]
        fn crate_constant_is_constant(constant in arb_uppercase_ident()) {
            prop_assume!(!constant.is_empty());

            let expr_str = format!("crate::{}", constant);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::empty();
                let result = classify_seed_expr(&expr, &args);

                prop_assert!(
                    result.is_ok(),
                    "crate::{} should parse successfully",
                    constant
                );
                prop_assert!(
                    matches!(result.unwrap(), ClassifiedSeed::Constant(_)),
                    "crate::{} should be classified as Constant",
                    constant
                );
            }
        }

        /// `crate::module::CONSTANT` should be classified as Constant.
        #[test]
        fn crate_module_constant_is_constant(
            module in arb_lowercase_ident(),
            constant in arb_uppercase_ident()
        ) {
            prop_assume!(!module.is_empty() && !constant.is_empty());

            let expr_str = format!("crate::{}::{}", module, constant);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::empty();
                let result = classify_seed_expr(&expr, &args);

                prop_assert!(
                    result.is_ok(),
                    "crate::{}::{} should parse successfully",
                    module, constant
                );
                prop_assert!(
                    matches!(result.unwrap(), ClassifiedSeed::Constant(_)),
                    "crate::{}::{} should be classified as Constant",
                    module, constant
                );
            }
        }

        /// `super::CONSTANT` should be classified as Constant.
        #[test]
        fn super_constant_is_constant(constant in arb_uppercase_ident()) {
            prop_assume!(!constant.is_empty());

            let expr_str = format!("super::{}", constant);
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                let args = InstructionArgSet::empty();
                let result = classify_seed_expr(&expr, &args);

                prop_assert!(
                    result.is_ok(),
                    "super::{} should parse successfully",
                    constant
                );
                prop_assert!(
                    matches!(result.unwrap(), ClassifiedSeed::Constant(_)),
                    "super::{} should be classified as Constant",
                    constant
                );
            }
        }
    }
}
