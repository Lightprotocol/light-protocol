//! Cross-deserialization security tests for Token and CMint accounts.
//! Verifies that account_type discriminator at byte 165 prevents confusion.
//!
//! With the new extension-based design:
//! - Token base struct is 165 bytes (SPL-compatible)
//! - Account type byte is at position 165 ONLY when extensions are present
//! - Compression info is stored in the Compressible extension

use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_compressible::{compression_info::CompressionInfo, rent::RentConfig};
use light_token_interface::state::{
    AccountState, BaseMint, CompressedMint, CompressedMintMetadata, CompressibleExtension,
    ExtensionStruct, Token, ACCOUNT_TYPE_MINT, ACCOUNT_TYPE_TOKEN_ACCOUNT,
};

const ACCOUNT_TYPE_OFFSET: usize = 165;

fn create_test_cmint() -> CompressedMint {
    CompressedMint {
        base: BaseMint {
            mint_authority: Some(Pubkey::new_from_array([1; 32])),
            supply: 1000,
            decimals: 6,
            is_initialized: true,
            freeze_authority: None,
        },
        metadata: CompressedMintMetadata {
            version: 3,
            mint: Pubkey::new_from_array([2; 32]),
            cmint_decompressed: false,
            mint_signer: [5u8; 32],
            bump: 255,
        },
        reserved: [0u8; 16],
        account_type: ACCOUNT_TYPE_MINT,
        compression: CompressionInfo {
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
        extensions: None,
    }
}

/// Create a test Token with Compressible extension
fn create_test_ctoken_with_extension() -> Token {
    Token {
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
    }
}

/// Create a simple Token without extensions (SPL-compatible 165 bytes)
fn create_test_ctoken_simple() -> Token {
    Token {
        mint: Pubkey::new_from_array([1; 32]),
        owner: Pubkey::new_from_array([2; 32]),
        amount: 1000,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: None,
    }
}

#[test]
fn test_account_type_byte_position() {
    let cmint = create_test_cmint();
    let cmint_bytes = cmint.try_to_vec().unwrap();
    assert_eq!(
        cmint_bytes[ACCOUNT_TYPE_OFFSET], 1,
        "CMint account_type should be 1"
    );

    // Token with extensions has account_type byte at position 165
    let token = create_test_ctoken_with_extension();
    let ctoken_bytes = token.try_to_vec().unwrap();
    assert_eq!(
        ctoken_bytes[ACCOUNT_TYPE_OFFSET], 2,
        "Token with extensions account_type should be 2"
    );
}

#[test]
fn test_ctoken_without_extensions_size() {
    // Token without extensions should be exactly 165 bytes (SPL Token Account size)
    let token = create_test_ctoken_simple();
    let ctoken_bytes = token.try_to_vec().unwrap();
    assert_eq!(
        ctoken_bytes.len(),
        165,
        "Token without extensions should be 165 bytes"
    );
}

#[test]
fn test_cmint_bytes_fail_zero_copy_checked_as_ctoken() {
    let cmint = create_test_cmint();
    let cmint_bytes = cmint.try_to_vec().unwrap();

    // Token zero_copy_at_checked verifies account_type == 2, should fail for CMint bytes
    let result = Token::zero_copy_at_checked(&cmint_bytes);
    assert!(
        result.is_err(),
        "CMint bytes should fail to parse as Token zero-copy checked"
    );
}

#[test]
fn test_ctoken_bytes_fail_zero_copy_checked_as_cmint() {
    let token = create_test_ctoken_with_extension();
    let ctoken_bytes = token.try_to_vec().unwrap();

    // CompressedMint zero_copy_at_checked verifies account_type == 1, should fail for Token bytes
    let result = CompressedMint::zero_copy_at_checked(&ctoken_bytes);
    assert!(
        result.is_err(),
        "Token bytes should fail to parse as CMint zero-copy checked"
    );
}

#[test]
fn test_ctoken_bytes_wrong_account_type_as_cmint() {
    let token = create_test_ctoken_with_extension();
    let ctoken_bytes = token.try_to_vec().unwrap();

    // Deserialize as CMint - should succeed but have wrong account_type
    let cmint = CompressedMint::try_from_slice(&ctoken_bytes);
    match cmint {
        Ok(mint) => {
            assert_ne!(
                mint.account_type, ACCOUNT_TYPE_MINT,
                "Cross-deserialized CMint should have wrong account_type"
            );
        }
        Err(_) => {
            // Also acceptable - deserialization failure
        }
    }
}

#[test]
fn test_cmint_bytes_borsh_as_ctoken() {
    let cmint = create_test_cmint();
    let cmint_bytes = cmint.try_to_vec().unwrap();

    // Try to deserialize CMint bytes as Token
    let result = Token::try_from_slice(&cmint_bytes);
    // Borsh deserialization is lenient, but checked deserialization should detect the wrong type
    match result {
        Ok(token) => {
            // Borsh is lenient and may succeed, but is_token_account() check should fail
            // because CMint has account_type = ACCOUNT_TYPE_MINT (1), not ACCOUNT_TYPE_TOKEN_ACCOUNT (2)
            assert!(
                !token.is_token_account(),
                "CMint bytes deserialized as Token should fail is_token_account() check"
            );
            assert_eq!(
                token.account_type(),
                ACCOUNT_TYPE_MINT,
                "CMint bytes should retain ACCOUNT_TYPE_MINT discriminator"
            );
        }
        Err(_) => {
            // Also acceptable - deserialization failure
        }
    }
}
