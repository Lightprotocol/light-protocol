//! Property-based tests for light_account_keywords functions.
//!
//! These tests verify correctness properties of:
//! - `is_standalone_keyword` - Standalone flag detection
//! - `is_shorthand_key` - Shorthand syntax eligibility
//! - `valid_keys_for_namespace` - Namespace key lookup
//! - `validate_namespaced_key` - Key validation with error messages

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::light_pdas::light_account_keywords::{
        is_shorthand_key, is_standalone_keyword, valid_keys_for_namespace, validate_namespaced_key,
        ASSOCIATED_TOKEN_NAMESPACE_KEYS, MINT_NAMESPACE_KEYS, SHORTHAND_KEYS_BY_NAMESPACE,
        STANDALONE_KEYWORDS, TOKEN_NAMESPACE_KEYS,
    };

    // ========================================================================
    // Strategies for generating test inputs
    // ========================================================================

    /// Strategy for generating known standalone keywords
    fn arb_standalone_keyword() -> impl Strategy<Value = &'static str> {
        prop::sample::select(STANDALONE_KEYWORDS)
    }

    /// Strategy for generating known token namespace keys
    fn arb_token_key() -> impl Strategy<Value = &'static str> {
        prop::sample::select(TOKEN_NAMESPACE_KEYS)
    }

    /// Strategy for generating known ATA namespace keys
    fn arb_ata_key() -> impl Strategy<Value = &'static str> {
        prop::sample::select(ASSOCIATED_TOKEN_NAMESPACE_KEYS)
    }

    /// Strategy for generating known mint namespace keys
    fn arb_mint_key() -> impl Strategy<Value = &'static str> {
        prop::sample::select(MINT_NAMESPACE_KEYS)
    }

    /// Strategy for generating random lowercase strings (likely invalid keys)
    fn arb_random_key() -> impl Strategy<Value = String> {
        "[a-z_]{1,20}"
    }

    /// Strategy for generating known valid namespaces
    fn arb_valid_namespace() -> impl Strategy<Value = &'static str> {
        prop::sample::select(vec!["token", "associated_token", "mint"])
    }

    // ========================================================================
    // Property Tests: is_standalone_keyword
    // ========================================================================

    proptest! {
        /// Known standalone keywords should be accepted.
        #[test]
        fn prop_known_keywords_accepted(keyword in arb_standalone_keyword()) {
            let result = is_standalone_keyword(keyword);
            prop_assert!(
                result,
                "Known standalone keyword '{}' should be accepted",
                keyword
            );
        }

        /// Random strings should (almost certainly) be rejected.
        #[test]
        fn prop_random_keywords_rejected(keyword in arb_random_key()) {
            // Skip if randomly generated one of the known keywords
            prop_assume!(!STANDALONE_KEYWORDS.contains(&keyword.as_str()));

            let result = is_standalone_keyword(&keyword);
            prop_assert!(
                !result,
                "Random string '{}' should be rejected as standalone keyword",
                keyword
            );
        }

        /// is_standalone_keyword should be deterministic.
        #[test]
        fn prop_standalone_deterministic(keyword in arb_random_key()) {
            let result1 = is_standalone_keyword(&keyword);
            let result2 = is_standalone_keyword(&keyword);
            prop_assert_eq!(
                result1, result2,
                "is_standalone_keyword should be deterministic for '{}'",
                keyword
            );
        }

        /// Keywords should be case-sensitive (uppercase should be rejected).
        #[test]
        fn prop_case_sensitive_uppercase(keyword in arb_standalone_keyword()) {
            let uppercase = keyword.to_uppercase();
            // Skip if uppercase happens to match (shouldn't for our keywords)
            prop_assume!(keyword != uppercase);

            let result = is_standalone_keyword(&uppercase);
            prop_assert!(
                !result,
                "Uppercase '{}' should be rejected (case sensitive)",
                uppercase
            );
        }
    }

    // ========================================================================
    // Property Tests: is_shorthand_key
    // ========================================================================

    proptest! {
        /// Token namespace shorthand keys should be correctly identified.
        #[test]
        fn prop_token_shorthand_correct(key in arb_token_key()) {
            let result = is_shorthand_key("token", key);
            // Only mint, owner, bump are shorthand in token namespace
            let expected = ["mint", "owner", "bump"].contains(&key);
            prop_assert_eq!(
                result, expected,
                "token::{} shorthand should be {}",
                key, expected
            );
        }

        /// ATA namespace shorthand keys should be correctly identified.
        #[test]
        fn prop_ata_shorthand_correct(key in arb_ata_key()) {
            let result = is_shorthand_key("associated_token", key);
            // All ATA keys support shorthand: authority, mint, bump
            let expected = ["authority", "mint", "bump"].contains(&key);
            prop_assert_eq!(
                result, expected,
                "associated_token::{} shorthand should be {}",
                key, expected
            );
        }

        /// Mint namespace should have no shorthand keys.
        #[test]
        fn prop_mint_no_shorthand(key in arb_mint_key()) {
            let result = is_shorthand_key("mint", key);
            prop_assert!(
                !result,
                "mint::{} should NOT support shorthand",
                key
            );
        }

        /// Unknown namespace should return false for any key.
        #[test]
        fn prop_unknown_namespace_false(key in arb_random_key()) {
            let result = is_shorthand_key("unknown_namespace", &key);
            prop_assert!(
                !result,
                "Unknown namespace should return false for any key '{}'",
                key
            );
        }

        /// is_shorthand_key should be deterministic.
        #[test]
        fn prop_shorthand_deterministic(
            namespace in arb_valid_namespace(),
            key in arb_random_key()
        ) {
            let result1 = is_shorthand_key(namespace, &key);
            let result2 = is_shorthand_key(namespace, &key);
            prop_assert_eq!(
                result1, result2,
                "is_shorthand_key should be deterministic for {}::{}",
                namespace, key
            );
        }

        /// All shorthand keys defined should be recognized.
        #[test]
        fn prop_all_defined_shorthand_recognized(_seed in 0u32..1000) {
            for (namespace, keys) in SHORTHAND_KEYS_BY_NAMESPACE {
                for key in *keys {
                    let result = is_shorthand_key(namespace, key);
                    prop_assert!(
                        result,
                        "Defined shorthand key {}::{} should be recognized",
                        namespace, key
                    );
                }
            }
        }
    }

    // ========================================================================
    // Property Tests: valid_keys_for_namespace
    // ========================================================================

    proptest! {
        /// Token namespace should return TOKEN_NAMESPACE_KEYS.
        #[test]
        fn prop_token_returns_correct_keys(_seed in 0u32..1000) {
            let keys = valid_keys_for_namespace("token");
            prop_assert_eq!(
                keys, TOKEN_NAMESPACE_KEYS,
                "token namespace should return TOKEN_NAMESPACE_KEYS"
            );
        }

        /// Associated token namespace should return ASSOCIATED_TOKEN_NAMESPACE_KEYS.
        #[test]
        fn prop_ata_returns_correct_keys(_seed in 0u32..1000) {
            let keys = valid_keys_for_namespace("associated_token");
            prop_assert_eq!(
                keys, ASSOCIATED_TOKEN_NAMESPACE_KEYS,
                "associated_token namespace should return ASSOCIATED_TOKEN_NAMESPACE_KEYS"
            );
        }

        /// Mint namespace should return MINT_NAMESPACE_KEYS.
        #[test]
        fn prop_mint_returns_correct_keys(_seed in 0u32..1000) {
            let keys = valid_keys_for_namespace("mint");
            prop_assert_eq!(
                keys, MINT_NAMESPACE_KEYS,
                "mint namespace should return MINT_NAMESPACE_KEYS"
            );
        }

        /// Unknown namespace should return empty slice.
        #[test]
        fn prop_unknown_namespace_empty(namespace in "[a-z]{5,10}") {
            // Skip if randomly generated a valid namespace
            prop_assume!(namespace != "token" && namespace != "associated_token" && namespace != "mint");

            let keys = valid_keys_for_namespace(&namespace);
            prop_assert!(
                keys.is_empty(),
                "Unknown namespace '{}' should return empty slice",
                namespace
            );
        }

        /// valid_keys_for_namespace should be deterministic.
        #[test]
        fn prop_valid_keys_deterministic(namespace in arb_valid_namespace()) {
            let keys1 = valid_keys_for_namespace(namespace);
            let keys2 = valid_keys_for_namespace(namespace);
            prop_assert_eq!(
                keys1, keys2,
                "valid_keys_for_namespace should be deterministic for '{}'",
                namespace
            );
        }
    }

    // ========================================================================
    // Property Tests: validate_namespaced_key
    // ========================================================================

    proptest! {
        /// All keys returned by valid_keys_for_namespace should validate successfully.
        #[test]
        fn prop_valid_keys_accepted(namespace in arb_valid_namespace()) {
            let valid_keys = valid_keys_for_namespace(namespace);
            for key in valid_keys {
                let result = validate_namespaced_key(namespace, key);
                prop_assert!(
                    result.is_ok(),
                    "Valid key {}::{} should be accepted",
                    namespace, key
                );
            }
        }

        /// Random keys not in valid set should be rejected.
        #[test]
        fn prop_invalid_keys_rejected(namespace in arb_valid_namespace(), key in arb_random_key()) {
            let valid_keys = valid_keys_for_namespace(namespace);
            // Skip if randomly generated a valid key
            prop_assume!(!valid_keys.contains(&key.as_str()));

            let result = validate_namespaced_key(namespace, &key);
            prop_assert!(
                result.is_err(),
                "Invalid key {}::{} should be rejected",
                namespace, key
            );
        }

        /// Unknown namespace should return error.
        #[test]
        fn prop_unknown_namespace_rejected(namespace in "[a-z]{5,10}", key in arb_random_key()) {
            // Skip if randomly generated a valid namespace
            prop_assume!(namespace != "token" && namespace != "associated_token" && namespace != "mint");

            let result = validate_namespaced_key(&namespace, &key);
            prop_assert!(
                result.is_err(),
                "Unknown namespace '{}' should return error",
                namespace
            );
        }

        /// Error message should contain the invalid key.
        #[test]
        fn prop_error_contains_key(namespace in arb_valid_namespace(), key in arb_random_key()) {
            let valid_keys = valid_keys_for_namespace(namespace);
            prop_assume!(!valid_keys.contains(&key.as_str()));

            let result = validate_namespaced_key(namespace, &key);
            if let Err(err_msg) = result {
                prop_assert!(
                    err_msg.contains(&key),
                    "Error message should contain the invalid key '{}', got: {}",
                    key, err_msg
                );
            }
        }

        /// Error message should suggest valid alternatives.
        #[test]
        fn prop_error_suggests_valid(namespace in arb_valid_namespace(), key in arb_random_key()) {
            let valid_keys = valid_keys_for_namespace(namespace);
            prop_assume!(!valid_keys.contains(&key.as_str()));

            let result = validate_namespaced_key(namespace, &key);
            if let Err(err_msg) = result {
                // At least one valid key should be mentioned in the error
                let contains_valid_key = valid_keys.iter().any(|vk| err_msg.contains(vk));
                prop_assert!(
                    contains_valid_key,
                    "Error message should suggest valid alternatives, got: {}",
                    err_msg
                );
            }
        }

        /// validate_namespaced_key should be deterministic.
        #[test]
        fn prop_validate_deterministic(namespace in arb_valid_namespace(), key in arb_random_key()) {
            let result1 = validate_namespaced_key(namespace, &key);
            let result2 = validate_namespaced_key(namespace, &key);
            prop_assert_eq!(
                result1, result2,
                "validate_namespaced_key should be deterministic for {}::{}",
                namespace, key
            );
        }
    }
}
