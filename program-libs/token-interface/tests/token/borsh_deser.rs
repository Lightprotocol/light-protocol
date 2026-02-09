//! Tests that borsh deserialization of Token rejects invalid account_type bytes.
//!
//! The fix in borsh.rs ensures that when an account_type byte is present (byte 165)
//! but does not equal ACCOUNT_TYPE_TOKEN_ACCOUNT (2), deserialization returns an error
//! instead of silently accepting the wrong type.

use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_compressible::{compression_info::CompressionInfo, rent::RentConfig};
use light_token_interface::state::{
    AccountState, CompressibleExtension, ExtensionStruct, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT,
};

/// Helper: create a valid Token with extensions, serialize it, then
/// tamper the account_type byte at offset 165 to the given value.
fn serialize_token_with_account_type(account_type_byte: u8) -> Vec<u8> {
    let token = Token {
        mint: Pubkey::new_from_array([1; 32]),
        owner: Pubkey::new_from_array([2; 32]),
        amount: 1000,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: Some(vec![ExtensionStruct::Compressible(CompressibleExtension {
            decimals_option: 1,
            decimals: 6,
            compression_only: false,
            is_ata: 0,
            info: CompressionInfo {
                config_account_version: 1,
                compress_to_pubkey: 0,
                account_version: 3,
                lamports_per_write: 100,
                compression_authority: [3u8; 32],
                rent_sponsor: [4u8; 32],
                last_claimed_slot: 100,
                rent_exemption_paid: 0,
                _reserved: 0,
                rent_config: RentConfig {
                    base_rent: 0,
                    compression_cost: 0,
                    lamports_per_byte_per_epoch: 0,
                    max_funded_epochs: 0,
                    max_top_up: 0,
                },
            },
        })]),
    };

    let mut bytes = token.try_to_vec().unwrap();
    // Tamper byte 165 (account_type) to the requested value
    bytes[165] = account_type_byte;
    bytes
}

#[test]
fn test_borsh_deser_rejects_mint_account_type() {
    // account_type = 1 (ACCOUNT_TYPE_MINT) should be rejected
    let bytes = serialize_token_with_account_type(1);
    let result = Token::try_from_slice(&bytes);
    assert!(
        result.is_err(),
        "Borsh deserialization should reject account_type=1 (Mint) for Token"
    );
}

#[test]
fn test_borsh_deser_rejects_zero_account_type() {
    // account_type = 0 (uninitialized / invalid) should be rejected
    let bytes = serialize_token_with_account_type(0);
    let result = Token::try_from_slice(&bytes);
    assert!(
        result.is_err(),
        "Borsh deserialization should reject account_type=0 for Token"
    );
}

#[test]
fn test_borsh_deser_rejects_arbitrary_account_type() {
    // account_type = 255 (arbitrary invalid value) should be rejected
    let bytes = serialize_token_with_account_type(255);
    let result = Token::try_from_slice(&bytes);
    assert!(
        result.is_err(),
        "Borsh deserialization should reject account_type=255 for Token"
    );
}

#[test]
fn test_borsh_deser_accepts_valid_token_account_type() {
    // account_type = 2 (ACCOUNT_TYPE_TOKEN_ACCOUNT) should succeed
    let bytes = serialize_token_with_account_type(ACCOUNT_TYPE_TOKEN_ACCOUNT);
    let result = Token::try_from_slice(&bytes);
    assert!(
        result.is_ok(),
        "Borsh deserialization should accept account_type=2 (Token)"
    );
}

#[test]
fn test_borsh_deser_accepts_base_token_without_extensions() {
    // A 165-byte base SPL token (no account_type byte) should still deserialize fine
    let token = Token {
        mint: Pubkey::new_from_array([1; 32]),
        owner: Pubkey::new_from_array([2; 32]),
        amount: 500,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: None,
    };

    let bytes = token.try_to_vec().unwrap();
    assert_eq!(bytes.len(), 165, "Base token should be 165 bytes");

    let deserialized = Token::try_from_slice(&bytes).expect("Should deserialize base token");
    assert_eq!(deserialized, token);
}

#[test]
fn test_borsh_deser_error_message_on_invalid_account_type() {
    let bytes = serialize_token_with_account_type(1);
    let err = Token::try_from_slice(&bytes).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(
        err.to_string()
            .contains("Account type does not match Token account"),
        "Error message should indicate account type mismatch, got: {}",
        err
    );
}
