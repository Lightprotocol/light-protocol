//! Cross-deserialization security tests for Token and Mint accounts.
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
    AccountState, BaseMint, CompressibleExtension, ExtensionStruct, Mint, MintMetadata, Token,
    ACCOUNT_TYPE_MINT, ACCOUNT_TYPE_TOKEN_ACCOUNT,
};

const ACCOUNT_TYPE_OFFSET: usize = 165;

fn create_test_mint() -> Mint {
    Mint {
        base: BaseMint {
            mint_authority: Some(Pubkey::new_from_array([1; 32])),
            supply: 1000,
            decimals: 6,
            is_initialized: true,
            freeze_authority: None,
        },
        metadata: MintMetadata {
            version: 3,
            mint: Pubkey::new_from_array([2; 32]),
            mint_decompressed: false,
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
    let mint = create_test_mint();
    let mint_bytes = mint.try_to_vec().unwrap();
    assert_eq!(
        mint_bytes[ACCOUNT_TYPE_OFFSET], 1,
        "Mint account_type should be 1"
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
fn test_mint_bytes_fail_zero_copy_checked_as_ctoken() {
    let mint = create_test_mint();
    let mint_bytes = mint.try_to_vec().unwrap();

    // Token zero_copy_at_checked verifies account_type == 2, should fail for Mint bytes
    let result = Token::zero_copy_at_checked(&mint_bytes);
    assert!(
        result.is_err(),
        "Mint bytes should fail to parse as Token zero-copy checked"
    );
}

#[test]
fn test_ctoken_bytes_fail_zero_copy_checked_as_mint() {
    let token = create_test_ctoken_with_extension();
    let ctoken_bytes = token.try_to_vec().unwrap();

    // Mint zero_copy_at_checked verifies account_type == 1, should fail for Token bytes
    let result = Mint::zero_copy_at_checked(&ctoken_bytes);
    assert!(
        result.is_err(),
        "Token bytes should fail to parse as Mint zero-copy checked"
    );
}

#[test]
fn test_ctoken_bytes_wrong_account_type_as_mint() {
    let token = create_test_ctoken_with_extension();
    let ctoken_bytes = token.try_to_vec().unwrap();

    // Deserialize as Mint - should succeed but have wrong account_type
    let mint = Mint::try_from_slice(&ctoken_bytes);
    match mint {
        Ok(mint) => {
            assert_ne!(
                mint.account_type, ACCOUNT_TYPE_MINT,
                "Cross-deserialized Mint should have wrong account_type"
            );
        }
        Err(_) => {
            // Also acceptable - deserialization failure
        }
    }
}

#[test]
fn test_mint_bytes_borsh_as_ctoken() {
    let mint = create_test_mint();
    let mint_bytes = mint.try_to_vec().unwrap();

    // Mint has account_type = ACCOUNT_TYPE_MINT (1) at byte 165.
    // Borsh deserialization of Token now rejects non-Token account_type bytes.
    let result = Token::try_from_slice(&mint_bytes);
    assert!(
        result.is_err(),
        "Mint bytes should fail borsh deserialization as Token due to account_type mismatch"
    );
}
