//! Shared keyword definitions for `#[light_account(...)]` attribute parsing.
//!
//! This module provides a single source of truth for valid keywords used in
//! `#[light_account(...)]` attributes. Keywords use Anchor-style `namespace::key` syntax.
//!
//! ## Syntax
//!
//! All attribute parameters (except type markers) require a namespace prefix:
//! - `token::authority`, `token::mint`, `token::owner`, `token::bump`
//! - `associated_token::authority`, `associated_token::mint`, `associated_token::bump`
//! - `mint::signer`, `mint::authority`, `mint::decimals`, `mint::seeds`, etc.
//!
//! ## Example
//!
//! ```ignore
//! #[light_account(init, token,
//!     token::authority = [VAULT_SEED, self.offer.key()],
//!     token::mint = token_mint_a,
//!     token::owner = authority,
//!     token::bump = params.vault_bump
//! )]
//! pub vault: UncheckedAccount<'info>,
//! ```

/// Valid keys for `token::` namespace in `#[light_account(token, ...)]` attributes.
/// These map to the TokenAccountField struct.
pub const TOKEN_NAMESPACE_KEYS: &[&str] = &["authority", "mint", "owner", "bump"];

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
    "rent_payment",
    "write_top_up",
    "name",
    "symbol",
    "uri",
    "update_authority",
    "additional_metadata",
];

/// Standalone keywords that don't require a value (flags).
/// These can appear as bare identifiers without `= value`.
pub const STANDALONE_KEYWORDS: &[&str] = &["init", "token", "associated_token", "mint"];

/// Keywords that support shorthand syntax within their namespace.
/// For example, `token::mint` alone is equivalent to `token::mint = mint`.
/// Maps namespace -> list of shorthand-eligible keys
pub const SHORTHAND_KEYS_BY_NAMESPACE: &[(&str, &[&str])] = &[
    ("token", &["mint", "owner", "bump"]),
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
            "Unknown namespace `{}`. Expected: token, associated_token, or mint",
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
            "Unknown namespace `{}`. Expected: token, associated_token, or mint",
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
