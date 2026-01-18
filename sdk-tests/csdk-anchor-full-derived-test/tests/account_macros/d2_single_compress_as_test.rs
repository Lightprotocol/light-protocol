//! D2 Tests: SingleCompressAsRecord trait derive tests
//!
//! Tests each trait derived by `RentFreeAccount` macro for `SingleCompressAsRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//! - CompressiblePack -> Pack + Unpack + PackedSingleCompressAsRecord

use super::shared::CompressibleTestFactory;
use crate::generate_trait_tests;
use csdk_anchor_full_derived_test::{PackedSingleCompressAsRecord, SingleCompressAsRecord};
use light_hasher::{DataHasher, Sha256};
use light_sdk::{
    compressible::{CompressAs, CompressionInfo, Pack},
    instruction::PackedAccounts,
};
use solana_pubkey::Pubkey;

// =============================================================================
// Factory Implementation
// =============================================================================

impl CompressibleTestFactory for SingleCompressAsRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: Some(CompressionInfo::default()),
            owner: Pubkey::new_unique(),
            cached: 999,
            counter: 0,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: None,
            owner: Pubkey::new_unique(),
            cached: 999,
            counter: 0,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(SingleCompressAsRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_overrides_cached_to_zero() {
    let owner = Pubkey::new_unique();
    let counter = 100u64;

    let record = SingleCompressAsRecord {
        compression_info: Some(CompressionInfo::default()),
        owner,
        cached: 999, // Original value
        counter,
    };

    let compressed = record.compress_as();
    // Per #[compress_as(cached = 0)], cached should be 0 in compressed form
    assert_eq!(compressed.cached, 0, "cached should be 0 after compress_as");
    // Other fields should be preserved
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.counter, counter);
}

#[test]
fn test_compress_as_preserves_counter() {
    let owner = Pubkey::new_unique();
    let counter = 555u64;

    let record = SingleCompressAsRecord {
        compression_info: Some(CompressionInfo::default()),
        owner,
        cached: 999,
        counter,
    };

    let compressed = record.compress_as();
    // counter has no compress_as override, should be preserved
    assert_eq!(compressed.counter, counter);
}

#[test]
fn test_compress_as_with_multiple_cached_values() {
    let owner = Pubkey::new_unique();

    for cached_val in &[0u64, 100, 999, u64::MAX] {
        let record = SingleCompressAsRecord {
            compression_info: Some(CompressionInfo::default()),
            owner,
            cached: *cached_val,
            counter: 0,
        };

        let compressed = record.compress_as();
        // All should compress cached to 0
        assert_eq!(compressed.cached, 0, "cached should always be 0 after compress_as");
    }
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_counter() {
    let owner = Pubkey::new_unique();

    let record1 = SingleCompressAsRecord {
        compression_info: None,
        owner,
        cached: 0,
        counter: 1,
    };

    let record2 = SingleCompressAsRecord {
        compression_info: None,
        owner,
        cached: 0,
        counter: 2,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different counter should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_cached() {
    let owner = Pubkey::new_unique();

    let record1 = SingleCompressAsRecord {
        compression_info: None,
        owner,
        cached: 1,
        counter: 0,
    };

    let record2 = SingleCompressAsRecord {
        compression_info: None,
        owner,
        cached: 2,
        counter: 0,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different cached value should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_owner() {
    let record1 = SingleCompressAsRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        cached: 100,
        counter: 100,
    };

    let record2 = SingleCompressAsRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        cached: 100,
        counter: 100,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different owner should produce different hash"
    );
}

// =============================================================================
// Pack/Unpack Tests (struct-specific, cannot be generic)
// =============================================================================

#[test]
fn test_packed_struct_has_u8_owner() {
    let packed = PackedSingleCompressAsRecord {
        compression_info: None,
        owner: 0,
        cached: 42,
        counter: 100,
    };

    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.cached, 42u64);
    assert_eq!(packed.counter, 100u64);
}

#[test]
fn test_pack_converts_pubkey_to_index() {
    let owner = Pubkey::new_unique();
    let record = SingleCompressAsRecord {
        compression_info: None,
        owner,
        cached: 50,
        counter: 100,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts);

    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.cached, 50);
    assert_eq!(packed.counter, 100);
}

#[test]
fn test_pack_reuses_same_pubkey_index() {
    let owner = Pubkey::new_unique();

    let record1 = SingleCompressAsRecord {
        compression_info: None,
        owner,
        cached: 1,
        counter: 1,
    };

    let record2 = SingleCompressAsRecord {
        compression_info: None,
        owner,
        cached: 2,
        counter: 2,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts);
    let packed2 = record2.pack(&mut packed_accounts);

    assert_eq!(
        packed1.owner, packed2.owner,
        "same pubkey should produce same index"
    );
}

#[test]
fn test_pack_different_pubkeys_get_different_indices() {
    let record1 = SingleCompressAsRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        cached: 1,
        counter: 1,
    };

    let record2 = SingleCompressAsRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        cached: 2,
        counter: 2,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts);
    let packed2 = record2.pack(&mut packed_accounts);

    assert_ne!(
        packed1.owner, packed2.owner,
        "different pubkeys should produce different indices"
    );
}

#[test]
fn test_pack_sets_compression_info_to_none() {
    let record_with_info = SingleCompressAsRecord {
        compression_info: Some(CompressionInfo::default()),
        owner: Pubkey::new_unique(),
        cached: 100,
        counter: 100,
    };

    let record_without_info = SingleCompressAsRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        cached: 200,
        counter: 200,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record_with_info.pack(&mut packed_accounts);
    let packed2 = record_without_info.pack(&mut packed_accounts);

    assert!(
        packed1.compression_info.is_none(),
        "pack should set compression_info to None"
    );
    assert!(
        packed2.compression_info.is_none(),
        "pack should set compression_info to None"
    );
}

#[test]
fn test_pack_stores_pubkeys_in_packed_accounts() {
    let owner1 = Pubkey::new_unique();
    let owner2 = Pubkey::new_unique();

    let record1 = SingleCompressAsRecord {
        compression_info: None,
        owner: owner1,
        cached: 1,
        counter: 1,
    };

    let record2 = SingleCompressAsRecord {
        compression_info: None,
        owner: owner2,
        cached: 2,
        counter: 2,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts);
    let packed2 = record2.pack(&mut packed_accounts);

    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 2, "should have 2 pubkeys stored");
    assert_eq!(
        stored_pubkeys[packed1.owner as usize], owner1,
        "first pubkey should match"
    );
    assert_eq!(
        stored_pubkeys[packed2.owner as usize], owner2,
        "second pubkey should match"
    );
}

#[test]
fn test_pack_index_assignment_order() {
    let mut packed_accounts = PackedAccounts::default();

    let owners: Vec<Pubkey> = (0..5).map(|_| Pubkey::new_unique()).collect();
    let mut indices = Vec::new();

    for owner in &owners {
        let record = SingleCompressAsRecord {
            compression_info: None,
            owner: *owner,
            cached: 0,
            counter: 0,
        };
        let packed = record.pack(&mut packed_accounts);
        indices.push(packed.owner);
    }

    assert_eq!(indices, vec![0, 1, 2, 3, 4], "indices should be sequential");
}
