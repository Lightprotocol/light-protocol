//! Contains functional zero copy tests for:
//! - ZeroCopyNew
//!
//! Tests:
//! 1. test_compressed_token_new_zero_copy - basic creation without extensions
//! 2. test_compressed_token_new_zero_copy_with_pausable_extension - with extension

use light_compressed_account::Pubkey;
use light_compressible::{compression_info::CompressionInfo, rent::RentConfig};
use light_ctoken_interface::state::{
    ctoken::{AccountState, CToken, CompressedTokenConfig, BASE_TOKEN_ACCOUNT_SIZE},
    extensions::{ExtensionStruct, ExtensionStructConfig, PausableAccountExtension},
};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyNew};

fn zeroed_compression_info() -> CompressionInfo {
    CompressionInfo {
        config_account_version: 0,
        compress_to_pubkey: 0,
        account_version: 0,
        lamports_per_write: 0,
        compression_authority: [0u8; 32],
        rent_sponsor: [0u8; 32],
        last_claimed_slot: 0,
        rent_config: RentConfig {
            base_rent: 0,
            compression_cost: 0,
            lamports_per_byte_per_epoch: 0,
            max_funded_epochs: 0,
            max_top_up: 0,
        },
    }
}

fn default_config() -> CompressedTokenConfig {
    CompressedTokenConfig {
        mint: Pubkey::default(),
        owner: Pubkey::default(),
        state: 1,
        compression_only: false,
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
    let expected = CToken {
        mint: Pubkey::default(),
        owner: Pubkey::default(),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized, // state: 1 from default_config
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: 2, // ACCOUNT_TYPE_TOKEN_ACCOUNT
        decimals: None,
        compression_only: false,
        compression: zeroed_compression_info(),
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
        account_type: 2, // ACCOUNT_TYPE_TOKEN_ACCOUNT
        decimals: None,
        compression_only: false,
        compression: zeroed_compression_info(),
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
