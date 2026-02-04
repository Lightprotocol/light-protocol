//! Shared keyword definitions for `#[light_account(...)]` attribute parsing.
//!
//! This module provides a single source of truth for valid keywords used in
//! `#[light_account(...)]` attributes. Keywords use Anchor-style `namespace::key` syntax.
//!
//! ## Syntax
//!
//! All attribute parameters (except type markers) require a namespace prefix:
//! - `token::seeds`, `token::mint`, `token::owner`, `token::bump`
//! - `associated_token::authority`, `associated_token::mint`, `associated_token::bump`
//! - `mint::signer`, `mint::authority`, `mint::decimals`, `mint::seeds`, etc.
//!
//! ## Example
//!
//! ```ignore
//! #[light_account(init,
//!     token::seeds = [VAULT_SEED, self.mint.key()],
//!     token::mint = token_mint,
//!     token::owner = vault_authority,
//!     token::bump = params.vault_bump
//! )]
//! pub vault: UncheckedAccount<'info>,
//! ```

/// Valid keys for `pda::` namespace in `#[light_account(pda::...)]` attributes.
/// Used by `#[derive(LightProgram)]` enum variants.
/// - `seeds`: PDA seeds for account derivation
/// - `zero_copy`: Flag indicating zero-copy deserialization
pub const PDA_NAMESPACE_KEYS: &[&str] = &["seeds", "zero_copy"];

/// Valid keys for `token::` namespace in `#[light_account(init, token::...)]` attributes.
/// These map to the TokenAccountField struct.
/// - `seeds`: Token account PDA seeds (for signing as the token account) - can be dynamic
/// - `owner_seeds`: Owner PDA seeds (for signing when owner is a PDA) - MUST BE CONSTANTS ONLY
pub const TOKEN_NAMESPACE_KEYS: &[&str] = &["seeds", "mint", "owner", "bump", "owner_seeds"];

/// Valid keys for `associated_token::` namespace in `#[light_account(associated_token, ...)]`.
/// Note: `authority` is the user-facing name (maps internally to `owner` in AtaField).
pub const ASSOCIATED_TOKEN_NAMESPACE_KEYS: &[&str] = &["authority", "mint", "bump"];

/// Valid keys for `mint::` namespace in `#[light_account(init, mint, ...)]` attributes.
pub const MINT_NAMESPACE_KEYS: &[&str] = &[
    "signer",
    "authority",
    "decimals",
    "seeds",
    "bump",
    "freeze_authority",
    "authority_seeds",
    "authority_bump",
    "name",
    "symbol",
    "uri",
    "update_authority",
    "additional_metadata",
];

/// Standalone keywords that don't require a value (flags).
/// Only `init` can appear as a truly standalone keyword.
/// `mint` and `zero_copy` are only valid after `init`.
/// `token` and `associated_token` require namespaced syntax.
pub const STANDALONE_KEYWORDS: &[&str] = &["init"];

/// Keywords that support shorthand syntax within their namespace.
/// For example, `token::mint` alone is equivalent to `token::mint = mint`.
/// Maps namespace -> list of shorthand-eligible keys
pub const SHORTHAND_KEYS_BY_NAMESPACE: &[(&str, &[&str])] = &[
    ("token", &["mint", "owner", "bump"]), // seeds requires array, no shorthand
    ("associated_token", &["authority", "mint", "bump"]),
    // mint namespace does not support shorthand - values are typically expressions
];

/// Check if a keyword is a standalone flag (doesn't require a value).
#[inline]
pub fn is_standalone_keyword(keyword: &str) -> bool {
    STANDALONE_KEYWORDS.contains(&keyword)
}

/// Check if a key supports shorthand syntax within a given namespace.
/// Shorthand means the key can appear without `= value` and defaults to `key = key`.
#[inline]
pub fn is_shorthand_key(namespace: &str, key: &str) -> bool {
    for (ns, keys) in SHORTHAND_KEYS_BY_NAMESPACE {
        if *ns == namespace {
            return keys.contains(&key);
        }
    }
    false
}

/// Get the valid keys for a given namespace.
///
/// # Arguments
/// * `namespace` - One of "token", "associated_token", or "mint"
///
/// # Returns
/// A slice of valid key strings for the namespace.
pub fn valid_keys_for_namespace(namespace: &str) -> &'static [&'static str] {
    match namespace {
        "pda" => PDA_NAMESPACE_KEYS,
        "token" => TOKEN_NAMESPACE_KEYS,
        "associated_token" => ASSOCIATED_TOKEN_NAMESPACE_KEYS,
        "mint" => MINT_NAMESPACE_KEYS,
        _ => &[],
    }
}

/// Validate a key within a namespace.
///
/// # Arguments
/// * `namespace` - The namespace (e.g., "token", "mint")
/// * `key` - The key within that namespace
///
/// # Returns
/// `Ok(())` if valid, `Err(error_message)` if invalid.
pub fn validate_namespaced_key(namespace: &str, key: &str) -> Result<(), String> {
    let valid_keys = valid_keys_for_namespace(namespace);

    if valid_keys.is_empty() {
        return Err(format!(
            "Unknown namespace `{}`. Expected: pda, token, associated_token, or mint",
            namespace
        ));
    }

    if !valid_keys.contains(&key) {
        return Err(format!(
            "Unknown key `{}` in `{}::` namespace. Allowed: {}",
            key,
            namespace,
            valid_keys.join(", ")
        ));
    }

    Ok(())
}

/// Generate an error message for an unknown key within a namespace.
///
/// # Arguments
/// * `namespace` - The namespace (e.g., "token", "mint")
/// * `key` - The unknown key that was encountered
///
/// # Returns
/// A formatted error message string.
pub fn unknown_key_error(namespace: &str, key: &str) -> String {
    let valid = valid_keys_for_namespace(namespace);
    if valid.is_empty() {
        format!(
            "Unknown namespace `{}`. Expected: pda, token, associated_token, or mint",
            namespace
        )
    } else {
        format!(
            "Unknown key `{}` in #[light_account({}, ...)]. Allowed for `{}::`: {}",
            key,
            namespace,
            namespace,
            valid.join(", ")
        )
    }
}

/// Generate an error message for a missing namespace prefix.
///
/// # Arguments
/// * `key` - The key that's missing a namespace prefix
/// * `account_type` - The account type context (e.g., "token", "mint")
///
/// # Returns
/// A formatted error message string with suggestions.
pub fn missing_namespace_error(key: &str, account_type: &str) -> String {
    format!(
        "Missing namespace prefix for `{}`. Use `{}::{}` instead of just `{}`",
        key, account_type, key, key
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pda_namespace_keys() {
        assert!(PDA_NAMESPACE_KEYS.contains(&"seeds"));
        assert!(PDA_NAMESPACE_KEYS.contains(&"zero_copy"));
        assert!(!PDA_NAMESPACE_KEYS.contains(&"unknown"));
    }

    #[test]
    fn test_token_namespace_keys() {
        assert!(TOKEN_NAMESPACE_KEYS.contains(&"seeds"));
        assert!(TOKEN_NAMESPACE_KEYS.contains(&"mint"));
        assert!(TOKEN_NAMESPACE_KEYS.contains(&"owner"));
        assert!(TOKEN_NAMESPACE_KEYS.contains(&"bump"));
        assert!(!TOKEN_NAMESPACE_KEYS.contains(&"authority")); // use seeds for token PDA
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
        // Only init is a true standalone keyword
        assert!(is_standalone_keyword("init"));
        // mint and zero_copy are only valid after init, not standalone
        assert!(!is_standalone_keyword("mint"));
        assert!(!is_standalone_keyword("zero_copy"));
        // token and associated_token require namespaced syntax
        assert!(!is_standalone_keyword("token"));
        assert!(!is_standalone_keyword("associated_token"));
        assert!(!is_standalone_keyword("authority"));
    }

    #[test]
    fn test_shorthand_keys() {
        // token namespace
        assert!(is_shorthand_key("token", "mint"));
        assert!(is_shorthand_key("token", "owner"));
        assert!(is_shorthand_key("token", "bump"));
        assert!(!is_shorthand_key("token", "seeds")); // seeds requires array

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
        let pda_kw = valid_keys_for_namespace("pda");
        assert_eq!(pda_kw, PDA_NAMESPACE_KEYS);

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
        assert!(validate_namespaced_key("token", "seeds").is_ok());
        assert!(validate_namespaced_key("token", "mint").is_ok());
        assert!(validate_namespaced_key("associated_token", "authority").is_ok());
        assert!(validate_namespaced_key("mint", "signer").is_ok());
        assert!(validate_namespaced_key("mint", "decimals").is_ok());

        // Invalid keys
        assert!(validate_namespaced_key("token", "invalid").is_err());
        assert!(validate_namespaced_key("token", "authority").is_err()); // use seeds for token
        assert!(validate_namespaced_key("unknown_namespace", "key").is_err());
    }

    #[test]
    fn test_unknown_key_error() {
        let error = unknown_key_error("token", "invalid");
        assert!(error.contains("invalid"));
        assert!(error.contains("token"));
        assert!(error.contains("seeds"));

        let error = unknown_key_error("unknown", "key");
        assert!(error.contains("Unknown namespace"));
    }

    #[test]
    fn test_missing_namespace_error() {
        let error = missing_namespace_error("authority", "token");
        assert!(error.contains("token::authority"));
        assert!(error.contains("Missing namespace prefix"));
    }
}
