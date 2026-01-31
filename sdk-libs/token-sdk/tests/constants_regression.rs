//! Hardcoded regression tests for protocol constants.
//!
//! These tests ensure that critical protocol constants remain stable across versions.
//! Any changes to these values would break compatibility with existing deployments.

use std::str::FromStr;

use light_token::constants::{
    config_pda, id, rent_sponsor_pda, LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_CPI_AUTHORITY,
    LIGHT_TOKEN_PROGRAM_ID, RENT_SPONSOR_V1, SPL_TOKEN_2022_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID,
};
use solana_pubkey::Pubkey;

#[test]
fn test_light_token_cpi_authority_hardcoded() {
    let expected = Pubkey::from_str("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy").unwrap();
    assert_eq!(
        LIGHT_TOKEN_CPI_AUTHORITY, expected,
        "LIGHT_TOKEN_CPI_AUTHORITY must match expected value"
    );
}

#[test]
fn test_compressible_config_v1_hardcoded() {
    let expected = Pubkey::from_str("ACXg8a7VaqecBWrSbdu73W4Pg9gsqXJ3EXAqkHyhvVXg").unwrap();
    assert_eq!(
        LIGHT_TOKEN_CONFIG, expected,
        "LIGHT_TOKEN_CONFIG must match expected value"
    );
}

#[test]
fn test_rent_sponsor_v1_hardcoded() {
    let expected = Pubkey::from_str("r18WwUxfG8kQ69bQPAB2jV6zGNKy3GosFGctjQoV4ti").unwrap();
    assert_eq!(
        RENT_SPONSOR_V1, expected,
        "RENT_SPONSOR_V1 must match expected value"
    );
}

#[test]
fn test_light_token_program_id_hardcoded() {
    let expected = Pubkey::from_str("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m").unwrap();
    assert_eq!(
        LIGHT_TOKEN_PROGRAM_ID, expected,
        "LIGHT_TOKEN_PROGRAM_ID must match expected value"
    );
}

#[test]
fn test_config_pda_returns_expected_value() {
    assert_eq!(
        config_pda(),
        LIGHT_TOKEN_CONFIG,
        "config_pda() must return LIGHT_TOKEN_CONFIG"
    );
}

#[test]
fn test_rent_sponsor_pda_returns_expected_value() {
    assert_eq!(
        rent_sponsor_pda(),
        RENT_SPONSOR_V1,
        "rent_sponsor_pda() must return RENT_SPONSOR_V1"
    );
}

#[test]
fn test_spl_token_program_ids_hardcoded() {
    let expected_spl_token =
        Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let expected_spl_token_2022 =
        Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap();

    assert_eq!(
        SPL_TOKEN_PROGRAM_ID, expected_spl_token,
        "SPL_TOKEN_PROGRAM_ID must match expected value"
    );
    assert_eq!(
        SPL_TOKEN_2022_PROGRAM_ID, expected_spl_token_2022,
        "SPL_TOKEN_2022_PROGRAM_ID must match expected value"
    );
}

#[test]
fn test_id_function_returns_program_id() {
    assert_eq!(
        id(),
        LIGHT_TOKEN_PROGRAM_ID,
        "id() must return LIGHT_TOKEN_PROGRAM_ID"
    );
}
