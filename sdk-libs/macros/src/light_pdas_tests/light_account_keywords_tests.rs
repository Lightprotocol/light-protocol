//! Unit tests for light_account keyword validation.
//!
//! Extracted from `light_pdas/light_account_keywords.rs`.

use crate::light_pdas::light_account_keywords::{
    is_shorthand_key, is_standalone_keyword, missing_namespace_error, unknown_key_error,
    valid_keys_for_namespace, validate_namespaced_key, ASSOCIATED_TOKEN_NAMESPACE_KEYS,
    MINT_NAMESPACE_KEYS, TOKEN_NAMESPACE_KEYS,
};

#[test]
fn test_token_namespace_keys() {
    assert!(TOKEN_NAMESPACE_KEYS.contains(&"authority"));
    assert!(TOKEN_NAMESPACE_KEYS.contains(&"mint"));
    assert!(TOKEN_NAMESPACE_KEYS.contains(&"owner"));
    assert!(TOKEN_NAMESPACE_KEYS.contains(&"bump"));
    assert!(!TOKEN_NAMESPACE_KEYS.contains(&"unknown"));
}

#[test]
fn test_associated_token_namespace_keys() {
    assert!(ASSOCIATED_TOKEN_NAMESPACE_KEYS.contains(&"authority"));
    assert!(ASSOCIATED_TOKEN_NAMESPACE_KEYS.contains(&"mint"));
    assert!(ASSOCIATED_TOKEN_NAMESPACE_KEYS.contains(&"bump"));
    assert!(!ASSOCIATED_TOKEN_NAMESPACE_KEYS.contains(&"owner")); // renamed to authority
    assert!(!ASSOCIATED_TOKEN_NAMESPACE_KEYS.contains(&"unknown"));
}

#[test]
fn test_mint_namespace_keys() {
    assert!(MINT_NAMESPACE_KEYS.contains(&"signer")); // renamed from mint_signer
    assert!(MINT_NAMESPACE_KEYS.contains(&"authority"));
    assert!(MINT_NAMESPACE_KEYS.contains(&"decimals"));
    assert!(MINT_NAMESPACE_KEYS.contains(&"seeds")); // renamed from mint_seeds
    assert!(MINT_NAMESPACE_KEYS.contains(&"bump")); // renamed from mint_bump
    assert!(MINT_NAMESPACE_KEYS.contains(&"freeze_authority"));
    assert!(MINT_NAMESPACE_KEYS.contains(&"authority_seeds"));
    assert!(MINT_NAMESPACE_KEYS.contains(&"authority_bump"));
    assert!(MINT_NAMESPACE_KEYS.contains(&"name"));
    assert!(MINT_NAMESPACE_KEYS.contains(&"symbol"));
    assert!(MINT_NAMESPACE_KEYS.contains(&"uri"));
    assert!(MINT_NAMESPACE_KEYS.contains(&"update_authority"));
    assert!(MINT_NAMESPACE_KEYS.contains(&"additional_metadata"));
}

#[test]
fn test_standalone_keywords() {
    assert!(is_standalone_keyword("init"));
    assert!(is_standalone_keyword("token"));
    assert!(is_standalone_keyword("associated_token"));
    assert!(is_standalone_keyword("mint"));
    assert!(!is_standalone_keyword("authority"));
}

#[test]
fn test_shorthand_keys() {
    // token namespace
    assert!(is_shorthand_key("token", "mint"));
    assert!(is_shorthand_key("token", "owner"));
    assert!(is_shorthand_key("token", "bump"));
    assert!(!is_shorthand_key("token", "authority")); // authority requires seeds array

    // associated_token namespace
    assert!(is_shorthand_key("associated_token", "authority"));
    assert!(is_shorthand_key("associated_token", "mint"));
    assert!(is_shorthand_key("associated_token", "bump"));

    // mint namespace - no shorthand
    assert!(!is_shorthand_key("mint", "signer"));
    assert!(!is_shorthand_key("mint", "authority"));
}

#[test]
fn test_valid_keys_for_namespace() {
    let token_kw = valid_keys_for_namespace("token");
    assert_eq!(token_kw, TOKEN_NAMESPACE_KEYS);

    let ata_kw = valid_keys_for_namespace("associated_token");
    assert_eq!(ata_kw, ASSOCIATED_TOKEN_NAMESPACE_KEYS);

    let mint_kw = valid_keys_for_namespace("mint");
    assert_eq!(mint_kw, MINT_NAMESPACE_KEYS);

    let unknown_kw = valid_keys_for_namespace("unknown");
    assert!(unknown_kw.is_empty());
}

#[test]
fn test_validate_namespaced_key() {
    // Valid keys
    assert!(validate_namespaced_key("token", "authority").is_ok());
    assert!(validate_namespaced_key("token", "mint").is_ok());
    assert!(validate_namespaced_key("associated_token", "authority").is_ok());
    assert!(validate_namespaced_key("mint", "signer").is_ok());
    assert!(validate_namespaced_key("mint", "decimals").is_ok());

    // Invalid keys
    assert!(validate_namespaced_key("token", "invalid").is_err());
    assert!(validate_namespaced_key("unknown_namespace", "key").is_err());
}

#[test]
fn test_unknown_key_error() {
    let error = unknown_key_error("token", "invalid");
    assert!(error.contains("invalid"));
    assert!(error.contains("token"));
    assert!(error.contains("authority"));

    let error = unknown_key_error("unknown", "key");
    assert!(error.contains("Unknown namespace"));
}

#[test]
fn test_missing_namespace_error() {
    let error = missing_namespace_error("authority", "token");
    assert!(error.contains("token::authority"));
    assert!(error.contains("Missing namespace prefix"));
}
