//! D2 Tests: AllCompressAsRecord trait derive tests
//!
//! Tests each trait derived by `RentFreeAccount` macro for `AllCompressAsRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//! - CompressiblePack -> Pack + Unpack + PackedAllCompressAsRecord

use super::shared::CompressibleTestFactory;
use crate::generate_trait_tests;
use csdk_anchor_full_derived_test::{AllCompressAsRecord, PackedAllCompressAsRecord};
use light_hasher::{DataHasher, Sha256};
use light_sdk::{
    compressible::{CompressAs, CompressionInfo, Pack},
    instruction::PackedAccounts,
};
use solana_pubkey::Pubkey;

// =============================================================================
// Factory Implementation
// =============================================================================

impl CompressibleTestFactory for AllCompressAsRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: Some(CompressionInfo::default()),
            owner: Pubkey::new_unique(),
            time: 999,
            score: 999,
            cached: 999,
            end: Some(999),
            counter: 0,
            flag: false,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: None,
            owner: Pubkey::new_unique(),
            time: 999,
            score: 999,
            cached: 999,
            end: Some(999),
            counter: 0,
            flag: false,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(AllCompressAsRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_overrides_numeric_fields() {
    let owner = Pubkey::new_unique();
    let counter = 100u64;
    let flag = true;

    let record = AllCompressAsRecord {
        compression_info: Some(CompressionInfo::default()),
        owner,
        time: 888,   // Original value
        score: 777,  // Original value
        cached: 666, // Original value
        end: Some(999),
        counter,
        flag,
    };

    let compressed = record.compress_as();

    // Per #[compress_as(time = 0, score = 0, cached = 0)]:
    assert_eq!(compressed.time, 0, "time should be 0 after compress_as");
    assert_eq!(compressed.score, 0, "score should be 0 after compress_as");
    assert_eq!(compressed.cached, 0, "cached should be 0 after compress_as");

    // Other fields should be preserved
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.counter, counter);
    assert_eq!(compressed.flag, flag);
}

#[test]
fn test_compress_as_overrides_option_to_none() {
    let owner = Pubkey::new_unique();
    let counter = 100u64;

    let record = AllCompressAsRecord {
        compression_info: Some(CompressionInfo::default()),
        owner,
        time: 100,
        score: 100,
        cached: 100,
        end: Some(999), // Original value
        counter,
        flag: false,
    };

    let compressed = record.compress_as();

    // Per #[compress_as(end = None)]:
    assert_eq!(compressed.end, None, "end should be None after compress_as");

    // Other fields should be correct
    assert_eq!(compressed.time, 0);
    assert_eq!(compressed.score, 0);
    assert_eq!(compressed.cached, 0);
}

#[test]
fn test_compress_as_preserves_non_overridden_fields() {
    let owner = Pubkey::new_unique();
    let counter = 555u64;
    let flag = true;

    let record = AllCompressAsRecord {
        compression_info: Some(CompressionInfo::default()),
        owner,
        time: 100,
        score: 200,
        cached: 300,
        end: Some(400),
        counter,
        flag,
    };

    let compressed = record.compress_as();

    // counter and flag have no compress_as override, should be preserved
    assert_eq!(compressed.counter, counter);
    assert_eq!(compressed.flag, flag);
    assert_eq!(compressed.owner, owner);
}

#[test]
fn test_compress_as_all_overrides_together() {
    let owner = Pubkey::new_unique();
    let counter = 777u64;
    let flag = false;

    let record = AllCompressAsRecord {
        compression_info: Some(CompressionInfo::default()),
        owner,
        time: u64::MAX,
        score: u64::MAX,
        cached: u64::MAX,
        end: Some(u64::MAX),
        counter,
        flag,
    };

    let compressed = record.compress_as();

    // All overridden fields should be at their override values
    assert_eq!(compressed.time, 0);
    assert_eq!(compressed.score, 0);
    assert_eq!(compressed.cached, 0);
    assert_eq!(compressed.end, None);

    // Non-overridden fields should be preserved
    assert_eq!(compressed.counter, counter);
    assert_eq!(compressed.flag, flag);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_counter() {
    let owner = Pubkey::new_unique();

    let record1 = AllCompressAsRecord {
        compression_info: None,
        owner,
        time: 0,
        score: 0,
        cached: 0,
        end: None,
        counter: 1,
        flag: false,
    };

    let record2 = AllCompressAsRecord {
        compression_info: None,
        owner,
        time: 0,
        score: 0,
        cached: 0,
        end: None,
        counter: 2,
        flag: false,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different counter should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_flag() {
    let owner = Pubkey::new_unique();

    let record1 = AllCompressAsRecord {
        compression_info: None,
        owner,
        time: 0,
        score: 0,
        cached: 0,
        end: None,
        counter: 0,
        flag: true,
    };

    let record2 = AllCompressAsRecord {
        compression_info: None,
        owner,
        time: 0,
        score: 0,
        cached: 0,
        end: None,
        counter: 0,
        flag: false,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different flag should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_time() {
    let owner = Pubkey::new_unique();

    let record1 = AllCompressAsRecord {
        compression_info: None,
        owner,
        time: 1,
        score: 0,
        cached: 0,
        end: None,
        counter: 0,
        flag: false,
    };

    let record2 = AllCompressAsRecord {
        compression_info: None,
        owner,
        time: 2,
        score: 0,
        cached: 0,
        end: None,
        counter: 0,
        flag: false,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different time should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_owner() {
    let record1 = AllCompressAsRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        time: 100,
        score: 100,
        cached: 100,
        end: Some(100),
        counter: 100,
        flag: false,
    };

    let record2 = AllCompressAsRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        time: 100,
        score: 100,
        cached: 100,
        end: Some(100),
        counter: 100,
        flag: false,
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
    let packed = PackedAllCompressAsRecord {
        compression_info: None,
        owner: 0,
        time: 42,
        score: 43,
        cached: 44,
        end: None,
        counter: 100,
        flag: true,
    };

    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.time, 42u64);
    assert_eq!(packed.score, 43u64);
    assert_eq!(packed.cached, 44u64);
    assert_eq!(packed.end, None);
    assert_eq!(packed.counter, 100u64);
    assert_eq!(packed.flag, true);
}

#[test]
fn test_pack_converts_pubkey_to_index() {
    let owner = Pubkey::new_unique();
    let record = AllCompressAsRecord {
        compression_info: None,
        owner,
        time: 50,
        score: 60,
        cached: 70,
        end: Some(80),
        counter: 100,
        flag: true,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts);

    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.time, 50);
    assert_eq!(packed.score, 60);
    assert_eq!(packed.cached, 70);
    assert_eq!(packed.end, Some(80));
    assert_eq!(packed.counter, 100);
    assert_eq!(packed.flag, true);
}

#[test]
fn test_pack_reuses_same_pubkey_index() {
    let owner = Pubkey::new_unique();

    let record1 = AllCompressAsRecord {
        compression_info: None,
        owner,
        time: 1,
        score: 1,
        cached: 1,
        end: Some(1),
        counter: 1,
        flag: true,
    };

    let record2 = AllCompressAsRecord {
        compression_info: None,
        owner,
        time: 2,
        score: 2,
        cached: 2,
        end: Some(2),
        counter: 2,
        flag: false,
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
    let record1 = AllCompressAsRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        time: 1,
        score: 1,
        cached: 1,
        end: Some(1),
        counter: 1,
        flag: true,
    };

    let record2 = AllCompressAsRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        time: 2,
        score: 2,
        cached: 2,
        end: Some(2),
        counter: 2,
        flag: false,
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
    let record_with_info = AllCompressAsRecord {
        compression_info: Some(CompressionInfo::default()),
        owner: Pubkey::new_unique(),
        time: 100,
        score: 100,
        cached: 100,
        end: Some(100),
        counter: 100,
        flag: true,
    };

    let record_without_info = AllCompressAsRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        time: 200,
        score: 200,
        cached: 200,
        end: Some(200),
        counter: 200,
        flag: false,
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

    let record1 = AllCompressAsRecord {
        compression_info: None,
        owner: owner1,
        time: 1,
        score: 1,
        cached: 1,
        end: Some(1),
        counter: 1,
        flag: true,
    };

    let record2 = AllCompressAsRecord {
        compression_info: None,
        owner: owner2,
        time: 2,
        score: 2,
        cached: 2,
        end: Some(2),
        counter: 2,
        flag: false,
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
        let record = AllCompressAsRecord {
            compression_info: None,
            owner: *owner,
            time: 0,
            score: 0,
            cached: 0,
            end: None,
            counter: 0,
            flag: false,
        };
        let packed = record.pack(&mut packed_accounts);
        indices.push(packed.owner);
    }

    assert_eq!(indices, vec![0, 1, 2, 3, 4], "indices should be sequential");
}
