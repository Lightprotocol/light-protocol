//! D2 Tests: MultipleCompressAsRecord trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `MultipleCompressAsRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace

use csdk_anchor_full_derived_test::{MultipleCompressAsRecord, PackedMultipleCompressAsRecord};
use light_account::{CompressAs, CompressionInfo, Pack};
use light_hasher::{DataHasher, Sha256};
use light_sdk::instruction::PackedAccounts;
use solana_pubkey::Pubkey;

use super::shared::CompressibleTestFactory;
use crate::generate_trait_tests;

// =============================================================================
// Factory Implementation
// =============================================================================

impl CompressibleTestFactory for MultipleCompressAsRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::default(),
            owner: Pubkey::new_unique(),
            start: 999,
            score: 999,
            cached: 999,
            counter: 0,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::compressed(),
            owner: Pubkey::new_unique(),
            start: 999,
            score: 999,
            cached: 999,
            counter: 0,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(MultipleCompressAsRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_overrides_all_marked_fields() {
    let owner = Pubkey::new_unique();
    let counter = 100u64;

    let record = MultipleCompressAsRecord {
        compression_info: CompressionInfo::default(),
        owner,
        start: 888,  // Original value
        score: 777,  // Original value
        cached: 666, // Original value
        counter,
    };

    let compressed = record.compress_as();

    // Per #[compress_as(start = 0, score = 0, cached = 0)]:
    assert_eq!(compressed.start, 0, "start should be 0 after compress_as");
    assert_eq!(compressed.score, 0, "score should be 0 after compress_as");
    assert_eq!(compressed.cached, 0, "cached should be 0 after compress_as");

    // Fields without compress_as override should be preserved
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.counter, counter);
}

#[test]
fn test_compress_as_preserves_non_overridden_fields() {
    let owner = Pubkey::new_unique();
    let counter = 555u64;

    let record = MultipleCompressAsRecord {
        compression_info: CompressionInfo::default(),
        owner,
        start: 100,
        score: 200,
        cached: 300,
        counter,
    };

    let compressed = record.compress_as();

    // counter has no compress_as override, should be preserved
    assert_eq!(compressed.counter, counter);
    assert_eq!(compressed.owner, owner);
}

#[test]
fn test_compress_as_with_all_max_values() {
    let owner = Pubkey::new_unique();

    let record = MultipleCompressAsRecord {
        compression_info: CompressionInfo::default(),
        owner,
        start: u64::MAX,
        score: u64::MAX,
        cached: u64::MAX,
        counter: u64::MAX,
    };

    let compressed = record.compress_as();

    // Overridden fields should still be 0
    assert_eq!(compressed.start, 0);
    assert_eq!(compressed.score, 0);
    assert_eq!(compressed.cached, 0);
    // Non-overridden fields should be preserved
    assert_eq!(compressed.counter, u64::MAX);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_counter() {
    let owner = Pubkey::new_unique();

    let record1 = MultipleCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start: 0,
        score: 0,
        cached: 0,
        counter: 1,
    };

    let record2 = MultipleCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start: 0,
        score: 0,
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
fn test_hash_differs_for_different_start() {
    let owner = Pubkey::new_unique();

    let record1 = MultipleCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start: 1,
        score: 0,
        cached: 0,
        counter: 0,
    };

    let record2 = MultipleCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start: 2,
        score: 0,
        cached: 0,
        counter: 0,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different start should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_score() {
    let owner = Pubkey::new_unique();

    let record1 = MultipleCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start: 0,
        score: 1,
        cached: 0,
        counter: 0,
    };

    let record2 = MultipleCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start: 0,
        score: 2,
        cached: 0,
        counter: 0,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different score should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_owner() {
    let record1 = MultipleCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        start: 100,
        score: 100,
        cached: 100,
        counter: 100,
    };

    let record2 = MultipleCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        start: 100,
        score: 100,
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
    let packed = PackedMultipleCompressAsRecord {
        owner: 0,
        start: 42,
        score: 43,
        cached: 44,
        counter: 100,
    };

    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.start, 42u64);
    assert_eq!(packed.score, 43u64);
    assert_eq!(packed.cached, 44u64);
    assert_eq!(packed.counter, 100u64);
}

#[test]
fn test_pack_converts_pubkey_to_index() {
    let owner = Pubkey::new_unique();
    let record = MultipleCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start: 50,
        score: 60,
        cached: 70,
        counter: 100,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.start, 50);
    assert_eq!(packed.score, 60);
    assert_eq!(packed.cached, 70);
    assert_eq!(packed.counter, 100);
}

#[test]
fn test_pack_reuses_same_pubkey_index() {
    let owner = Pubkey::new_unique();

    let record1 = MultipleCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start: 1,
        score: 1,
        cached: 1,
        counter: 1,
    };

    let record2 = MultipleCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start: 2,
        score: 2,
        cached: 2,
        counter: 2,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts).unwrap();
    let packed2 = record2.pack(&mut packed_accounts).unwrap();

    assert_eq!(
        packed1.owner, packed2.owner,
        "same pubkey should produce same index"
    );
}

#[test]
fn test_pack_different_pubkeys_get_different_indices() {
    let record1 = MultipleCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        start: 1,
        score: 1,
        cached: 1,
        counter: 1,
    };

    let record2 = MultipleCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        start: 2,
        score: 2,
        cached: 2,
        counter: 2,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts).unwrap();
    let packed2 = record2.pack(&mut packed_accounts).unwrap();

    assert_ne!(
        packed1.owner, packed2.owner,
        "different pubkeys should produce different indices"
    );
}

#[test]
fn test_pack_stores_pubkeys_in_packed_accounts() {
    let owner1 = Pubkey::new_unique();
    let owner2 = Pubkey::new_unique();

    let record1 = MultipleCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: owner1,
        start: 1,
        score: 1,
        cached: 1,
        counter: 1,
    };

    let record2 = MultipleCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: owner2,
        start: 2,
        score: 2,
        cached: 2,
        counter: 2,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts).unwrap();
    let packed2 = record2.pack(&mut packed_accounts).unwrap();

    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 2, "should have 2 pubkeys stored");
    assert_eq!(
        stored_pubkeys[packed1.owner as usize],
        owner1.to_bytes(),
        "first pubkey should match"
    );
    assert_eq!(
        stored_pubkeys[packed2.owner as usize],
        owner2.to_bytes(),
        "second pubkey should match"
    );
}

#[test]
fn test_pack_index_assignment_order() {
    let mut packed_accounts = PackedAccounts::default();

    let owners: Vec<Pubkey> = (0..5).map(|_| Pubkey::new_unique()).collect();
    let mut indices = Vec::new();

    for owner in &owners {
        let record = MultipleCompressAsRecord {
            compression_info: CompressionInfo::compressed(),
            owner: *owner,
            start: 0,
            score: 0,
            cached: 0,
            counter: 0,
        };
        let packed = record.pack(&mut packed_accounts).unwrap();
        indices.push(packed.owner);
    }

    assert_eq!(indices, vec![0, 1, 2, 3, 4], "indices should be sequential");
}
