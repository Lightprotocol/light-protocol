//! D2 Tests: OptionNoneCompressAsRecord trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `OptionNoneCompressAsRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace

use csdk_anchor_full_derived_test::{OptionNoneCompressAsRecord, PackedOptionNoneCompressAsRecord};
use light_account::{CompressAs, CompressionInfo, Pack};
use light_hasher::{DataHasher, Sha256};
use light_sdk::instruction::PackedAccounts;
use solana_pubkey::Pubkey;

use super::shared::CompressibleTestFactory;
use crate::generate_trait_tests;

// =============================================================================
// Factory Implementation
// =============================================================================

impl CompressibleTestFactory for OptionNoneCompressAsRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::default(),
            owner: Pubkey::new_unique(),
            start_time: 0,
            end_time: Some(999),
            counter: 0,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::compressed(),
            owner: Pubkey::new_unique(),
            start_time: 0,
            end_time: Some(999),
            counter: 0,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(OptionNoneCompressAsRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_overrides_end_time_to_none() {
    let owner = Pubkey::new_unique();
    let start_time = 100u64;
    let counter = 50u64;

    let record = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::default(),
        owner,
        start_time,
        end_time: Some(999), // Original value
        counter,
    };

    let compressed = record.compress_as();

    // Per #[compress_as(end_time = None)], end_time should be None in compressed form
    assert_eq!(
        compressed.end_time, None,
        "end_time should be None after compress_as"
    );
    // Other fields should be preserved
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.start_time, start_time);
    assert_eq!(compressed.counter, counter);
}

#[test]
fn test_compress_as_with_end_time_already_none() {
    let owner = Pubkey::new_unique();
    let start_time = 200u64;
    let counter = 75u64;

    let record = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::default(),
        owner,
        start_time,
        end_time: None, // Already None
        counter,
    };

    let compressed = record.compress_as();

    // Should remain None
    assert_eq!(compressed.end_time, None);
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.start_time, start_time);
    assert_eq!(compressed.counter, counter);
}

#[test]
fn test_compress_as_preserves_start_time_and_counter() {
    let owner = Pubkey::new_unique();
    let start_time = 555u64;
    let counter = 777u64;

    let record = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::default(),
        owner,
        start_time,
        end_time: Some(u64::MAX),
        counter,
    };

    let compressed = record.compress_as();

    // start_time and counter have no compress_as override, should be preserved
    assert_eq!(compressed.start_time, start_time);
    assert_eq!(compressed.counter, counter);
    // end_time should be None
    assert_eq!(compressed.end_time, None);
}

#[test]
fn test_compress_as_with_various_end_time_values() {
    let owner = Pubkey::new_unique();

    for end_val in &[Some(0u64), Some(100), Some(999), Some(u64::MAX), None] {
        let record = OptionNoneCompressAsRecord {
            compression_info: CompressionInfo::default(),
            owner,
            start_time: 0,
            end_time: *end_val,
            counter: 0,
        };

        let compressed = record.compress_as();
        // All should compress end_time to None
        assert_eq!(
            compressed.end_time, None,
            "end_time should always be None after compress_as"
        );
    }
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_counter() {
    let owner = Pubkey::new_unique();

    let record1 = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start_time: 0,
        end_time: None,
        counter: 1,
    };

    let record2 = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start_time: 0,
        end_time: None,
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
fn test_hash_differs_for_different_start_time() {
    let owner = Pubkey::new_unique();

    let record1 = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start_time: 1,
        end_time: None,
        counter: 0,
    };

    let record2 = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start_time: 2,
        end_time: None,
        counter: 0,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different start_time should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_end_time() {
    let owner = Pubkey::new_unique();

    let record1 = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start_time: 0,
        end_time: Some(1),
        counter: 0,
    };

    let record2 = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start_time: 0,
        end_time: Some(2),
        counter: 0,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different end_time should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_owner() {
    let record1 = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        start_time: 100,
        end_time: Some(100),
        counter: 100,
    };

    let record2 = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        start_time: 100,
        end_time: Some(100),
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
    let packed = PackedOptionNoneCompressAsRecord {
        owner: 0,
        start_time: 42,
        end_time: None,
        counter: 100,
    };

    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.start_time, 42u64);
    assert_eq!(packed.end_time, None);
    assert_eq!(packed.counter, 100u64);
}

#[test]
fn test_pack_converts_pubkey_to_index() {
    let owner = Pubkey::new_unique();
    let record = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start_time: 50,
        end_time: Some(100),
        counter: 100,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.start_time, 50);
    assert_eq!(packed.end_time, Some(100));
    assert_eq!(packed.counter, 100);
}

#[test]
fn test_pack_reuses_same_pubkey_index() {
    let owner = Pubkey::new_unique();

    let record1 = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start_time: 1,
        end_time: Some(1),
        counter: 1,
    };

    let record2 = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        start_time: 2,
        end_time: Some(2),
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
    let record1 = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        start_time: 1,
        end_time: Some(1),
        counter: 1,
    };

    let record2 = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        start_time: 2,
        end_time: Some(2),
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

    let record1 = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: owner1,
        start_time: 1,
        end_time: Some(1),
        counter: 1,
    };

    let record2 = OptionNoneCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: owner2,
        start_time: 2,
        end_time: Some(2),
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
        let record = OptionNoneCompressAsRecord {
            compression_info: CompressionInfo::compressed(),
            owner: *owner,
            start_time: 0,
            end_time: None,
            counter: 0,
        };
        let packed = record.pack(&mut packed_accounts).unwrap();
        indices.push(packed.owner);
    }

    assert_eq!(indices, vec![0, 1, 2, 3, 4], "indices should be sequential");
}
