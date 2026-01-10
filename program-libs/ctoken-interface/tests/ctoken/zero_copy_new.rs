//! Contains functional zero copy tests for:
//! - ZeroCopyNew
//!
//! Tests:
//! 1. test_compressed_token_new_zero_copy - basic creation without extensions
//! 2. test_compressed_token_new_zero_copy_with_pausable_extension - with extension

use light_compressed_account::Pubkey;
use light_ctoken_interface::state::{
    ctoken::{
        AccountState, CToken, CompressedTokenConfig, ACCOUNT_TYPE_TOKEN_ACCOUNT,
        BASE_TOKEN_ACCOUNT_SIZE,
    },
    extensions::{ExtensionStruct, ExtensionStructConfig, PausableAccountExtension},
};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyNew};

fn default_config() -> CompressedTokenConfig {
    CompressedTokenConfig {
        mint: Pubkey::default(),
        owner: Pubkey::default(),
        state: 1,
        extensions: None,
    }
}

#[test]
fn test_compressed_token_new_zero_copy() {
    let config = default_config();

    let required_size = CToken::byte_len(&config).unwrap();
    assert_eq!(required_size, BASE_TOKEN_ACCOUNT_SIZE as usize);

    let mut buffer = vec![0u8; required_size];
    let _ = CToken::new_zero_copy(&mut buffer, config).expect("Failed to initialize");

    let (zctoken, remaining) = CToken::zero_copy_at(&buffer).unwrap();

    // new_zero_copy now sets fields from config
    // Without extensions, CToken has SPL-compatible base layout only
    let expected = CToken {
        mint: Pubkey::default(),
        owner: Pubkey::default(),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized, // state: 1 from default_config
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: None,
    };

    assert_eq!(remaining.len(), 0);
    assert_eq!(zctoken, expected);
}

#[test]
fn test_compressed_token_new_zero_copy_with_pausable_extension() {
    let config = CompressedTokenConfig {
        extensions: Some(vec![ExtensionStructConfig::PausableAccount(())]),
        ..default_config()
    };

    let required_size = CToken::byte_len(&config).unwrap();
    assert!(required_size > BASE_TOKEN_ACCOUNT_SIZE as usize);

    let mut buffer = vec![0u8; required_size];
    let _ = CToken::new_zero_copy(&mut buffer, config).expect("Failed to initialize");

    let (zctoken, remaining) = CToken::zero_copy_at(&buffer).unwrap();

    // new_zero_copy now sets fields from config
    let expected = CToken {
        mint: Pubkey::default(),
        owner: Pubkey::default(),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized, // state: 1 from default_config
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: Some(vec![ExtensionStruct::PausableAccount(
            PausableAccountExtension,
        )]),
    };

    assert_eq!(remaining.len(), 0);
    assert_eq!(zctoken, expected);
}

#[test]
fn test_compressed_token_byte_len_consistency() {
    // No extensions
    let config_no_ext = default_config();
    let size_no_ext = CToken::byte_len(&config_no_ext).unwrap();
    let mut buffer_no_ext = vec![0u8; size_no_ext];
    let (_, remaining) = CToken::new_zero_copy(&mut buffer_no_ext, config_no_ext).unwrap();
    assert_eq!(remaining.len(), 0);

    // With pausable extension
    let config_with_ext = CompressedTokenConfig {
        extensions: Some(vec![ExtensionStructConfig::PausableAccount(())]),
        ..default_config()
    };
    let size_with_ext = CToken::byte_len(&config_with_ext).unwrap();
    let mut buffer_with_ext = vec![0u8; size_with_ext];
    let (_, remaining) = CToken::new_zero_copy(&mut buffer_with_ext, config_with_ext).unwrap();
    assert_eq!(remaining.len(), 0);

    assert!(size_with_ext > size_no_ext);
}

#[test]
fn test_new_zero_copy_fails_if_already_initialized() {
    let config = default_config();
    let required_size = CToken::byte_len(&config).unwrap();
    let mut buffer = vec![0u8; required_size];

    // First initialization should succeed
    let _ = CToken::new_zero_copy(&mut buffer, config.clone()).expect("First init should succeed");

    // Second initialization should fail because account is already initialized
    let result = CToken::new_zero_copy(&mut buffer, config);
    assert!(
        result.is_err(),
        "new_zero_copy should fail if account is already initialized"
    );
    assert_eq!(
        result.unwrap_err(),
        light_zero_copy::errors::ZeroCopyError::MemoryNotZeroed
    );
}
