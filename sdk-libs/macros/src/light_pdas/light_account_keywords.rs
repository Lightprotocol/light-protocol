//! Shared keyword definitions for `#[light_account(...)]` attribute parsing.
//!
//! This module provides a single source of truth for valid keywords used in
//! `#[light_account(...)]` attributes across both:
//! - `accounts/light_account.rs` - Used by `#[derive(LightAccounts)]`
//! - `account/seed_extraction.rs` - Used by `#[light_program]`

/// Valid keywords for `#[light_account(token, ...)]` attributes.
pub const TOKEN_KEYWORDS: &[&str] = &["authority", "mint", "owner", "bump"];

/// Valid keywords for `#[light_account(associated_token, ...)]` attributes.
pub const ASSOCIATED_TOKEN_KEYWORDS: &[&str] = &["owner", "mint", "bump"];

/// Standalone keywords that don't require a value (flags).
/// These can appear as bare identifiers without `= value`.
pub const STANDALONE_KEYWORDS: &[&str] = &["init", "token", "associated_token"];

/// Keywords that support shorthand syntax (key alone means key = key).
/// For example, `mint` alone is equivalent to `mint = mint`.
pub const SHORTHAND_KEYWORDS: &[&str] = &["mint", "owner", "bump"];

/// Check if a keyword is a standalone flag (doesn't require a value).
#[inline]
pub fn is_standalone_keyword(keyword: &str) -> bool {
    STANDALONE_KEYWORDS.contains(&keyword)
}

/// Check if a keyword supports shorthand syntax.
#[inline]
pub fn is_shorthand_keyword(keyword: &str) -> bool {
    SHORTHAND_KEYWORDS.contains(&keyword)
}

/// Get the valid keywords for a given account type.
///
/// # Arguments
/// * `account_type` - Either "token" or "associated_token"
///
/// # Returns
/// A slice of valid keyword strings for the account type.
pub fn valid_keywords_for_type(account_type: &str) -> &'static [&'static str] {
    match account_type {
        "token" => TOKEN_KEYWORDS,
        "associated_token" => ASSOCIATED_TOKEN_KEYWORDS,
        _ => &[],
    }
}

/// Generate an error message for an unknown keyword.
///
/// # Arguments
/// * `keyword` - The unknown keyword that was encountered
/// * `account_type` - The account type being parsed ("token" or "associated_token")
///
/// # Returns
/// A formatted error message string.
pub fn unknown_keyword_error(keyword: &str, account_type: &str) -> String {
    let valid = valid_keywords_for_type(account_type);
    format!(
        "Unknown argument `{}` in #[light_account({}, ...)]. Allowed: {}",
        keyword,
        account_type,
        valid.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_keywords() {
        assert!(TOKEN_KEYWORDS.contains(&"authority"));
        assert!(TOKEN_KEYWORDS.contains(&"mint"));
        assert!(TOKEN_KEYWORDS.contains(&"owner"));
        assert!(TOKEN_KEYWORDS.contains(&"bump")); // bump is now a valid token keyword
        assert!(!TOKEN_KEYWORDS.contains(&"unknown"));
    }

    #[test]
    fn test_ata_keywords() {
        assert!(ASSOCIATED_TOKEN_KEYWORDS.contains(&"owner"));
        assert!(ASSOCIATED_TOKEN_KEYWORDS.contains(&"mint"));
        assert!(ASSOCIATED_TOKEN_KEYWORDS.contains(&"bump"));
        assert!(!ASSOCIATED_TOKEN_KEYWORDS.contains(&"authority"));
        assert!(!ASSOCIATED_TOKEN_KEYWORDS.contains(&"unknown"));
    }

    #[test]
    fn test_standalone_keywords() {
        assert!(is_standalone_keyword("init"));
        assert!(is_standalone_keyword("token"));
        assert!(is_standalone_keyword("associated_token"));
        assert!(!is_standalone_keyword("authority"));
        assert!(!is_standalone_keyword("mint"));
    }

    #[test]
    fn test_shorthand_keywords() {
        assert!(is_shorthand_keyword("mint"));
        assert!(is_shorthand_keyword("owner"));
        assert!(is_shorthand_keyword("bump"));
        assert!(!is_shorthand_keyword("authority"));
        assert!(!is_shorthand_keyword("init"));
    }

    #[test]
    fn test_valid_keywords_for_type() {
        let token_kw = valid_keywords_for_type("token");
        assert_eq!(token_kw, TOKEN_KEYWORDS);

        let ata_kw = valid_keywords_for_type("associated_token");
        assert_eq!(ata_kw, ASSOCIATED_TOKEN_KEYWORDS);

        let unknown_kw = valid_keywords_for_type("unknown");
        assert!(unknown_kw.is_empty());
    }

    #[test]
    fn test_cross_validation() {
        // mint and owner are valid for both token and ATA
        assert!(TOKEN_KEYWORDS.contains(&"mint"));
        assert!(ASSOCIATED_TOKEN_KEYWORDS.contains(&"mint"));
        assert!(TOKEN_KEYWORDS.contains(&"owner"));
        assert!(ASSOCIATED_TOKEN_KEYWORDS.contains(&"owner"));
    }

    #[test]
    fn test_unknown_keyword_error() {
        let error = unknown_keyword_error("unknown_key", "token");
        assert!(error.contains("unknown_key"));
        assert!(error.contains("token"));
        assert!(error.contains("authority"));
        assert!(error.contains("mint"));
        assert!(error.contains("owner"));

        let error = unknown_keyword_error("bad", "associated_token");
        assert!(error.contains("bad"));
        assert!(error.contains("associated_token"));
        assert!(error.contains("bump"));
    }
}
