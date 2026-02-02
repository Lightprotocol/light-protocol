//! AMM PoolState Tests: PoolState trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `PoolState`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//! - CompressiblePack -> Pack + Unpack + PackedPoolState
//!
//! PoolState has 10 Pubkey fields and multiple numeric fields, testing
//! comprehensive Pack/Unpack behavior with multiple pubkey indices.

use csdk_anchor_full_derived_test::{PackedPoolState, PoolState};
use light_hasher::{DataHasher, Sha256};
use light_sdk::{
    compressible::{CompressAs, CompressionInfo, Pack},
    instruction::PackedAccounts,
};
use solana_pubkey::Pubkey;

use super::shared::CompressibleTestFactory;
use crate::generate_trait_tests;

// =============================================================================
// Factory Implementation
// =============================================================================

impl CompressibleTestFactory for PoolState {
    fn with_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::default(),
            amm_config: Pubkey::new_unique(),
            pool_creator: Pubkey::new_unique(),
            token_0_vault: Pubkey::new_unique(),
            token_1_vault: Pubkey::new_unique(),
            lp_mint: Pubkey::new_unique(),
            token_0_mint: Pubkey::new_unique(),
            token_1_mint: Pubkey::new_unique(),
            token_0_program: Pubkey::new_unique(),
            token_1_program: Pubkey::new_unique(),
            observation_key: Pubkey::new_unique(),
            auth_bump: 0,
            status: 0,
            lp_mint_decimals: 9,
            mint_0_decimals: 9,
            mint_1_decimals: 6,
            lp_supply: 0,
            protocol_fees_token_0: 0,
            protocol_fees_token_1: 0,
            fund_fees_token_0: 0,
            fund_fees_token_1: 0,
            open_time: 0,
            recent_epoch: 0,
            padding: [0u64; 1],
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::compressed(),
            amm_config: Pubkey::new_unique(),
            pool_creator: Pubkey::new_unique(),
            token_0_vault: Pubkey::new_unique(),
            token_1_vault: Pubkey::new_unique(),
            lp_mint: Pubkey::new_unique(),
            token_0_mint: Pubkey::new_unique(),
            token_1_mint: Pubkey::new_unique(),
            token_0_program: Pubkey::new_unique(),
            token_1_program: Pubkey::new_unique(),
            observation_key: Pubkey::new_unique(),
            auth_bump: 0,
            status: 0,
            lp_mint_decimals: 9,
            mint_0_decimals: 9,
            mint_1_decimals: 6,
            lp_supply: 0,
            protocol_fees_token_0: 0,
            protocol_fees_token_1: 0,
            fund_fees_token_0: 0,
            fund_fees_token_1: 0,
            open_time: 0,
            recent_epoch: 0,
            padding: [0u64; 1],
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(PoolState);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_preserves_numeric_fields() {
    let pool = PoolState {
        compression_info: CompressionInfo::default(),
        amm_config: Pubkey::new_unique(),
        pool_creator: Pubkey::new_unique(),
        token_0_vault: Pubkey::new_unique(),
        token_1_vault: Pubkey::new_unique(),
        lp_mint: Pubkey::new_unique(),
        token_0_mint: Pubkey::new_unique(),
        token_1_mint: Pubkey::new_unique(),
        token_0_program: Pubkey::new_unique(),
        token_1_program: Pubkey::new_unique(),
        observation_key: Pubkey::new_unique(),
        auth_bump: 42,
        status: 1,
        lp_mint_decimals: 8,
        mint_0_decimals: 6,
        mint_1_decimals: 9,
        lp_supply: 1000000,
        protocol_fees_token_0: 500,
        protocol_fees_token_1: 600,
        fund_fees_token_0: 100,
        fund_fees_token_1: 200,
        open_time: 1234567890,
        recent_epoch: 500,
        padding: [0u64; 1],
    };

    let compressed = pool.compress_as();
    let inner = compressed.into_owned();

    assert_eq!(inner.auth_bump, 42);
    assert_eq!(inner.status, 1);
    assert_eq!(inner.lp_mint_decimals, 8);
    assert_eq!(inner.mint_0_decimals, 6);
    assert_eq!(inner.mint_1_decimals, 9);
    assert_eq!(inner.lp_supply, 1000000);
    assert_eq!(inner.protocol_fees_token_0, 500);
    assert_eq!(inner.protocol_fees_token_1, 600);
    assert_eq!(inner.fund_fees_token_0, 100);
    assert_eq!(inner.fund_fees_token_1, 200);
    assert_eq!(inner.open_time, 1234567890);
    assert_eq!(inner.recent_epoch, 500);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_amm_config() {
    let mut pool1 = PoolState::without_compression_info();
    let mut pool2 = PoolState::without_compression_info();

    pool1.amm_config = Pubkey::new_unique();
    pool2.amm_config = Pubkey::new_unique();

    let hash1 = pool1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = pool2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different amm_config should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_lp_supply() {
    let mut pool1 = PoolState::without_compression_info();
    let mut pool2 = PoolState::without_compression_info();

    pool1.lp_supply = 1000000;
    pool2.lp_supply = 2000000;

    let hash1 = pool1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = pool2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different lp_supply should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_auth_bump() {
    let mut pool1 = PoolState::without_compression_info();
    let mut pool2 = PoolState::without_compression_info();

    pool1.auth_bump = 100;
    pool2.auth_bump = 200;

    let hash1 = pool1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = pool2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different auth_bump should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_open_time() {
    let mut pool1 = PoolState::without_compression_info();
    let mut pool2 = PoolState::without_compression_info();

    pool1.open_time = 1000;
    pool2.open_time = 2000;

    let hash1 = pool1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = pool2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different open_time should produce different hash"
    );
}

// =============================================================================
// Pack/Unpack Tests (struct-specific, cannot be generic)
// =============================================================================

#[test]
fn test_packed_struct_has_u8_pubkey_indices() {
    // PoolState has 10 Pubkey fields, so PackedPoolState should have 10 u8 fields
    let packed = PackedPoolState {
        amm_config: 0,
        pool_creator: 1,
        token_0_vault: 2,
        token_1_vault: 3,
        lp_mint: 4,
        token_0_mint: 5,
        token_1_mint: 6,
        token_0_program: 7,
        token_1_program: 8,
        observation_key: 9,
        auth_bump: 42,
        status: 0,
        lp_mint_decimals: 9,
        mint_0_decimals: 9,
        mint_1_decimals: 6,
        lp_supply: 100,
        protocol_fees_token_0: 0,
        protocol_fees_token_1: 0,
        fund_fees_token_0: 0,
        fund_fees_token_1: 0,
        open_time: 0,
        recent_epoch: 0,
        padding: [0u64; 1],
    };

    assert_eq!(packed.amm_config, 0u8);
    assert_eq!(packed.pool_creator, 1u8);
    assert_eq!(packed.observation_key, 9u8);
    assert_eq!(packed.auth_bump, 42u8);
}

#[test]
fn test_pack_converts_all_10_pubkeys_to_indices() {
    let pubkeys = vec![
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    ];

    let pool = PoolState {
        compression_info: CompressionInfo::compressed(),
        amm_config: pubkeys[0],
        pool_creator: pubkeys[1],
        token_0_vault: pubkeys[2],
        token_1_vault: pubkeys[3],
        lp_mint: pubkeys[4],
        token_0_mint: pubkeys[5],
        token_1_mint: pubkeys[6],
        token_0_program: pubkeys[7],
        token_1_program: pubkeys[8],
        observation_key: pubkeys[9],
        auth_bump: 0,
        status: 0,
        lp_mint_decimals: 9,
        mint_0_decimals: 9,
        mint_1_decimals: 6,
        lp_supply: 0,
        protocol_fees_token_0: 0,
        protocol_fees_token_1: 0,
        fund_fees_token_0: 0,
        fund_fees_token_1: 0,
        open_time: 0,
        recent_epoch: 0,
        padding: [0u64; 1],
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = pool.pack(&mut packed_accounts).unwrap();

    // All 10 pubkeys should have been added and assigned indices 0-9
    assert_eq!(packed.amm_config, 0u8);
    assert_eq!(packed.pool_creator, 1u8);
    assert_eq!(packed.token_0_vault, 2u8);
    assert_eq!(packed.token_1_vault, 3u8);
    assert_eq!(packed.lp_mint, 4u8);
    assert_eq!(packed.token_0_mint, 5u8);
    assert_eq!(packed.token_1_mint, 6u8);
    assert_eq!(packed.token_0_program, 7u8);
    assert_eq!(packed.token_1_program, 8u8);
    assert_eq!(packed.observation_key, 9u8);

    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 10);
    for (i, pubkey) in pubkeys.iter().enumerate() {
        assert_eq!(stored_pubkeys[i], pubkey.to_bytes());
    }
}

#[test]
fn test_pack_reuses_same_pubkey_indices() {
    // If the same pubkey is used in multiple fields, it should get the same index
    let shared_pubkey = Pubkey::new_unique();

    let pool = PoolState {
        compression_info: CompressionInfo::compressed(),
        amm_config: shared_pubkey,
        pool_creator: shared_pubkey,
        token_0_vault: Pubkey::new_unique(),
        token_1_vault: Pubkey::new_unique(),
        lp_mint: Pubkey::new_unique(),
        token_0_mint: Pubkey::new_unique(),
        token_1_mint: Pubkey::new_unique(),
        token_0_program: Pubkey::new_unique(),
        token_1_program: Pubkey::new_unique(),
        observation_key: Pubkey::new_unique(),
        auth_bump: 0,
        status: 0,
        lp_mint_decimals: 9,
        mint_0_decimals: 9,
        mint_1_decimals: 6,
        lp_supply: 0,
        protocol_fees_token_0: 0,
        protocol_fees_token_1: 0,
        fund_fees_token_0: 0,
        fund_fees_token_1: 0,
        open_time: 0,
        recent_epoch: 0,
        padding: [0u64; 1],
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = pool.pack(&mut packed_accounts).unwrap();

    // Same pubkey should get same index
    assert_eq!(
        packed.amm_config, packed.pool_creator,
        "same pubkey should produce same index"
    );
}

#[test]
fn test_pack_preserves_numeric_fields() {
    let pool = PoolState {
        compression_info: CompressionInfo::compressed(),
        amm_config: Pubkey::new_unique(),
        pool_creator: Pubkey::new_unique(),
        token_0_vault: Pubkey::new_unique(),
        token_1_vault: Pubkey::new_unique(),
        lp_mint: Pubkey::new_unique(),
        token_0_mint: Pubkey::new_unique(),
        token_1_mint: Pubkey::new_unique(),
        token_0_program: Pubkey::new_unique(),
        token_1_program: Pubkey::new_unique(),
        observation_key: Pubkey::new_unique(),
        auth_bump: 127,
        status: 2,
        lp_mint_decimals: 8,
        mint_0_decimals: 6,
        mint_1_decimals: 9,
        lp_supply: 9999999,
        protocol_fees_token_0: 444,
        protocol_fees_token_1: 555,
        fund_fees_token_0: 111,
        fund_fees_token_1: 222,
        open_time: 1700000000,
        recent_epoch: 999,
        padding: [42u64; 1],
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = pool.pack(&mut packed_accounts).unwrap();

    assert_eq!(packed.auth_bump, 127);
    assert_eq!(packed.status, 2);
    assert_eq!(packed.lp_mint_decimals, 8);
    assert_eq!(packed.mint_0_decimals, 6);
    assert_eq!(packed.mint_1_decimals, 9);
    assert_eq!(packed.lp_supply, 9999999);
    assert_eq!(packed.protocol_fees_token_0, 444);
    assert_eq!(packed.protocol_fees_token_1, 555);
    assert_eq!(packed.fund_fees_token_0, 111);
    assert_eq!(packed.fund_fees_token_1, 222);
    assert_eq!(packed.open_time, 1700000000);
    assert_eq!(packed.recent_epoch, 999);
    assert_eq!(packed.padding[0], 42);
}

#[test]
fn test_pack_different_pubkeys_get_different_indices() {
    let pool1 = PoolState {
        compression_info: CompressionInfo::compressed(),
        amm_config: Pubkey::new_unique(),
        pool_creator: Pubkey::new_unique(),
        token_0_vault: Pubkey::new_unique(),
        token_1_vault: Pubkey::new_unique(),
        lp_mint: Pubkey::new_unique(),
        token_0_mint: Pubkey::new_unique(),
        token_1_mint: Pubkey::new_unique(),
        token_0_program: Pubkey::new_unique(),
        token_1_program: Pubkey::new_unique(),
        observation_key: Pubkey::new_unique(),
        auth_bump: 0,
        status: 0,
        lp_mint_decimals: 9,
        mint_0_decimals: 9,
        mint_1_decimals: 6,
        lp_supply: 0,
        protocol_fees_token_0: 0,
        protocol_fees_token_1: 0,
        fund_fees_token_0: 0,
        fund_fees_token_1: 0,
        open_time: 0,
        recent_epoch: 0,
        padding: [0u64; 1],
    };

    let pool2 = PoolState {
        compression_info: CompressionInfo::compressed(),
        amm_config: Pubkey::new_unique(),
        pool_creator: Pubkey::new_unique(),
        token_0_vault: Pubkey::new_unique(),
        token_1_vault: Pubkey::new_unique(),
        lp_mint: Pubkey::new_unique(),
        token_0_mint: Pubkey::new_unique(),
        token_1_mint: Pubkey::new_unique(),
        token_0_program: Pubkey::new_unique(),
        token_1_program: Pubkey::new_unique(),
        observation_key: Pubkey::new_unique(),
        auth_bump: 0,
        status: 0,
        lp_mint_decimals: 9,
        mint_0_decimals: 9,
        mint_1_decimals: 6,
        lp_supply: 0,
        protocol_fees_token_0: 0,
        protocol_fees_token_1: 0,
        fund_fees_token_0: 0,
        fund_fees_token_1: 0,
        open_time: 0,
        recent_epoch: 0,
        padding: [0u64; 1],
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = pool1.pack(&mut packed_accounts).unwrap();
    let packed2 = pool2.pack(&mut packed_accounts).unwrap();

    // Different pubkeys should get different indices
    assert_ne!(
        packed1.amm_config, packed2.amm_config,
        "different pubkeys should produce different indices"
    );
}

#[test]
fn test_pack_stores_all_pubkeys_in_packed_accounts() {
    let pubkeys = vec![
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    ];

    let pool = PoolState {
        compression_info: CompressionInfo::compressed(),
        amm_config: pubkeys[0],
        pool_creator: pubkeys[1],
        token_0_vault: pubkeys[2],
        token_1_vault: pubkeys[3],
        lp_mint: pubkeys[4],
        token_0_mint: pubkeys[5],
        token_1_mint: pubkeys[6],
        token_0_program: pubkeys[7],
        token_1_program: pubkeys[8],
        observation_key: pubkeys[9],
        auth_bump: 0,
        status: 0,
        lp_mint_decimals: 9,
        mint_0_decimals: 9,
        mint_1_decimals: 6,
        lp_supply: 0,
        protocol_fees_token_0: 0,
        protocol_fees_token_1: 0,
        fund_fees_token_0: 0,
        fund_fees_token_1: 0,
        open_time: 0,
        recent_epoch: 0,
        padding: [0u64; 1],
    };

    let mut packed_accounts = PackedAccounts::default();
    let _packed = pool.pack(&mut packed_accounts).unwrap();

    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 10, "should have 10 pubkeys stored");

    // Verify each pubkey is stored at its index
    for (i, expected_pubkey) in pubkeys.iter().enumerate() {
        assert_eq!(
            stored_pubkeys[i], expected_pubkey.to_bytes(),
            "pubkey at index {} should match",
            i
        );
    }
}
