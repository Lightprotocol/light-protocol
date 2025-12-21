//! Cross-deserialization security tests for CToken and CMint accounts.
//! Verifies that account_type discriminator at byte 165 prevents confusion.

use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_compressible::{compression_info::CompressionInfo, rent::RentConfig};
use light_ctoken_interface::state::{
    AccountState, BaseMint, CToken, CompressedMint, CompressedMintMetadata, CompressibleExtension,
    ExtensionStruct, ACCOUNT_TYPE_MINT, ACCOUNT_TYPE_TOKEN_ACCOUNT,
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
        },
        reserved: [0u8; 49],
        account_type: ACCOUNT_TYPE_MINT,
        extensions: None,
    }
}

fn create_test_ctoken() -> CToken {
    CToken {
        mint: Pubkey::new_from_array([1; 32]),
        owner: Pubkey::new_from_array([2; 32]),
        amount: 1000,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        decimals: Some(6),
        compression_only: false,
        compression: CompressionInfo {
            config_account_version: 1,
            compress_to_pubkey: 0,
            account_version: 3,
            lamports_per_write: 100,
            compression_authority: [3u8; 32],
            rent_sponsor: [4u8; 32],
            last_claimed_slot: 100,
            rent_config: RentConfig {
                base_rent: 0,
                compression_cost: 0,
                lamports_per_byte_per_epoch: 0,
                max_funded_epochs: 0,
                max_top_up: 0,
            },
        },
        extensions: Some(vec![ExtensionStruct::Compressible(CompressibleExtension {
            compression_only: false,
            decimals: 6,
            has_decimals: 1,
            info: CompressionInfo {
                config_account_version: 1,
                compress_to_pubkey: 0,
                account_version: 3,
                lamports_per_write: 100,
                compression_authority: [3u8; 32],
                rent_sponsor: [4u8; 32],
                last_claimed_slot: 100,
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

#[test]
fn test_account_type_byte_position() {
    let cmint = create_test_cmint();
    let cmint_bytes = cmint.try_to_vec().unwrap();
    assert_eq!(
        cmint_bytes[ACCOUNT_TYPE_OFFSET], 1,
        "CMint account_type should be 1"
    );

    let ctoken = create_test_ctoken();
    let ctoken_bytes = ctoken.try_to_vec().unwrap();
    assert_eq!(
        ctoken_bytes[ACCOUNT_TYPE_OFFSET], 2,
        "CToken account_type should be 2"
    );
}

#[test]
fn test_cmint_bytes_fail_zero_copy_checked_as_ctoken() {
    let cmint = create_test_cmint();
    let cmint_bytes = cmint.try_to_vec().unwrap();

    // CToken zero_copy_at_checked verifies account_type == 2, should fail for CMint bytes
    let result = CToken::zero_copy_at_checked(&cmint_bytes);
    assert!(
        result.is_err(),
        "CMint bytes should fail to parse as CToken zero-copy checked"
    );
}

#[test]
fn test_ctoken_bytes_fail_zero_copy_checked_as_cmint() {
    let ctoken = create_test_ctoken();
    let ctoken_bytes = ctoken.try_to_vec().unwrap();

    // CompressedMint zero_copy_at_checked verifies account_type == 1, should fail for CToken bytes
    let result = CompressedMint::zero_copy_at_checked(&ctoken_bytes);
    assert!(
        result.is_err(),
        "CToken bytes should fail to parse as CMint zero-copy checked"
    );
}

#[test]
fn test_ctoken_bytes_wrong_account_type_as_cmint() {
    let ctoken = create_test_ctoken();
    let ctoken_bytes = ctoken.try_to_vec().unwrap();

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

    // Try to deserialize CMint bytes as CToken
    let result = CToken::try_from_slice(&cmint_bytes);
    // Should fail or produce invalid state
    match result {
        Ok(_ctoken) => {
            // If it succeeds, the data should be garbage/misaligned
            // CMint has different layout than CToken
            panic!("CMint bytes should not successfully parse as CToken");
        }
        Err(_) => {
            // Expected - deserialization should fail
        }
    }
}
