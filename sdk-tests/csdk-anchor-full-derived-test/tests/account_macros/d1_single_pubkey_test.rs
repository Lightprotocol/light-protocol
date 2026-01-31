//! D1 Tests: SinglePubkeyRecord trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `SinglePubkeyRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace

use csdk_anchor_full_derived_test::{PackedSinglePubkeyRecord, SinglePubkeyRecord};
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

impl CompressibleTestFactory for SinglePubkeyRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::default(),
            owner: Pubkey::new_unique(),
            counter: 0,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::compressed(),
            owner: Pubkey::new_unique(),
            counter: 0,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(SinglePubkeyRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_preserves_other_fields() {
    let owner = Pubkey::new_unique();
    let counter = 999u64;

    let record = SinglePubkeyRecord {
        compression_info: CompressionInfo::default(),
        owner,
        counter,
    };

    let compressed = record.compress_as();
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.counter, counter);
}

#[test]
fn test_compress_as_when_compression_info_already_none() {
    let owner = Pubkey::new_unique();
    let counter = 123u64;

    let record = SinglePubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        counter,
    };

    let compressed = record.compress_as();

    // Should still work and preserve fields
    assert_eq!(
        compressed.compression_info.state,
        light_sdk::compressible::CompressionState::Compressed
    );
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.counter, counter);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_counter() {
    let owner = Pubkey::new_unique();

    let record1 = SinglePubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        counter: 1,
    };

    let record2 = SinglePubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
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
fn test_hash_differs_for_different_owner() {
    let record1 = SinglePubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        counter: 100,
    };

    let record2 = SinglePubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
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
    // Verify PackedSinglePubkeyRecord has the expected structure
    // The Packed struct uses the same field name but changes type to u8
    // Note: PackedSinglePubkeyRecord no longer has compression_info field
    let packed = PackedSinglePubkeyRecord {
        owner: 0,
        counter: 42,
    };

    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.counter, 42u64);
}

#[test]
fn test_pack_converts_pubkey_to_index() {
    let owner = Pubkey::new_unique();
    let record = SinglePubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        counter: 100,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    // The owner should have been added to packed_accounts
    // and packed.owner should be the index (0 for first pubkey)
    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.counter, 100);

    let mut packed_accounts = PackedAccounts::default();
    packed_accounts.insert_or_get(Pubkey::new_unique());
    let packed = record.pack(&mut packed_accounts).unwrap();

    // The owner should have been added to packed_accounts
    // and packed.owner should be the index (1 for second pubkey)
    assert_eq!(packed.owner, 1u8);
    assert_eq!(packed.counter, 100);
}

#[test]
fn test_pack_reuses_same_pubkey_index() {
    let owner = Pubkey::new_unique();

    let record1 = SinglePubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        counter: 1,
    };

    let record2 = SinglePubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        counter: 2,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts).unwrap();
    let packed2 = record2.pack(&mut packed_accounts).unwrap();

    // Same pubkey should get same index
    assert_eq!(
        packed1.owner, packed2.owner,
        "same pubkey should produce same index"
    );
}

#[test]
fn test_pack_different_pubkeys_get_different_indices() {
    let record1 = SinglePubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        counter: 1,
    };

    let record2 = SinglePubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        counter: 2,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts).unwrap();
    let packed2 = record2.pack(&mut packed_accounts).unwrap();

    // Different pubkeys should get different indices
    assert_ne!(
        packed1.owner, packed2.owner,
        "different pubkeys should produce different indices"
    );
}

#[test]
fn test_pack_stores_pubkeys_in_packed_accounts() {
    let owner1 = Pubkey::new_unique();
    let owner2 = Pubkey::new_unique();

    let record1 = SinglePubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner: owner1,
        counter: 1,
    };

    let record2 = SinglePubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        owner: owner2,
        counter: 2,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts).unwrap();
    let packed2 = record2.pack(&mut packed_accounts).unwrap();

    // Verify pubkeys are stored and retrievable
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

    // Pack records with unique pubkeys in sequence
    let owners: Vec<Pubkey> = (0..5).map(|_| Pubkey::new_unique()).collect();
    let mut indices = Vec::new();

    for owner in &owners {
        let record = SinglePubkeyRecord {
            compression_info: CompressionInfo::compressed(),
            owner: *owner,
            counter: 0,
        };
        let packed = record.pack(&mut packed_accounts).unwrap();
        indices.push(packed.owner);
    }

    // Verify indices are assigned sequentially: 0, 1, 2, 3, 4
    assert_eq!(indices, vec![0, 1, 2, 3, 4], "indices should be sequential");
}
