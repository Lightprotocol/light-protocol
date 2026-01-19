//! Core Tests: PlaceholderRecord trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `PlaceholderRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//! - CompressiblePack -> Pack + Unpack + PackedPlaceholderRecord

use csdk_anchor_full_derived_test::{PackedPlaceholderRecord, PlaceholderRecord};
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

impl CompressibleTestFactory for PlaceholderRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: Some(CompressionInfo::default()),
            owner: Pubkey::new_unique(),
            name: "test placeholder".to_string(),
            placeholder_id: 1,
            counter: 0,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: None,
            owner: Pubkey::new_unique(),
            name: "test placeholder".to_string(),
            placeholder_id: 1,
            counter: 0,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(PlaceholderRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_preserves_other_fields() {
    let owner = Pubkey::new_unique();
    let name = "test placeholder".to_string();
    let placeholder_id = 42u64;
    let counter = 999u32;

    let record = PlaceholderRecord {
        compression_info: Some(CompressionInfo::default()),
        owner,
        name: name.clone(),
        placeholder_id,
        counter,
    };

    let compressed = record.compress_as();
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.name, name);
    assert_eq!(compressed.placeholder_id, placeholder_id);
    assert_eq!(compressed.counter, counter);
}

#[test]
fn test_compress_as_when_compression_info_already_none() {
    let owner = Pubkey::new_unique();
    let name = "test placeholder".to_string();
    let placeholder_id = 5u64;
    let counter = 123u32;

    let record = PlaceholderRecord {
        compression_info: None,
        owner,
        name: name.clone(),
        placeholder_id,
        counter,
    };

    let compressed = record.compress_as();

    // Should still work and preserve fields
    assert!(compressed.compression_info.is_none());
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.name, name);
    assert_eq!(compressed.placeholder_id, placeholder_id);
    assert_eq!(compressed.counter, counter);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_owner() {
    let record1 = PlaceholderRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        name: "test placeholder".to_string(),
        placeholder_id: 1,
        counter: 0,
    };

    let record2 = PlaceholderRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        name: "test placeholder".to_string(),
        placeholder_id: 1,
        counter: 0,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different owner should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_name() {
    let owner = Pubkey::new_unique();

    let record1 = PlaceholderRecord {
        compression_info: None,
        owner,
        name: "placeholder1".to_string(),
        placeholder_id: 1,
        counter: 0,
    };

    let record2 = PlaceholderRecord {
        compression_info: None,
        owner,
        name: "placeholder2".to_string(),
        placeholder_id: 1,
        counter: 0,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(hash1, hash2, "different name should produce different hash");
}

#[test]
fn test_hash_differs_for_different_placeholder_id() {
    let owner = Pubkey::new_unique();

    let record1 = PlaceholderRecord {
        compression_info: None,
        owner,
        name: "test placeholder".to_string(),
        placeholder_id: 1,
        counter: 0,
    };

    let record2 = PlaceholderRecord {
        compression_info: None,
        owner,
        name: "test placeholder".to_string(),
        placeholder_id: 2,
        counter: 0,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different placeholder_id should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_counter() {
    let owner = Pubkey::new_unique();

    let record1 = PlaceholderRecord {
        compression_info: None,
        owner,
        name: "test placeholder".to_string(),
        placeholder_id: 1,
        counter: 0,
    };

    let record2 = PlaceholderRecord {
        compression_info: None,
        owner,
        name: "test placeholder".to_string(),
        placeholder_id: 1,
        counter: 1,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different counter should produce different hash"
    );
}

// =============================================================================
// Pack/Unpack Tests (struct-specific, cannot be generic)
// =============================================================================

#[test]
fn test_packed_struct_has_u8_owner() {
    // Verify PackedPlaceholderRecord has the expected structure
    // The Packed struct uses the same field name but changes type to u8
    let packed = PackedPlaceholderRecord {
        compression_info: None,
        owner: 0,
        name: "test".to_string(),
        placeholder_id: 1,
        counter: 42,
    };

    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.placeholder_id, 1u64);
    assert_eq!(packed.counter, 42u32);
}

#[test]
fn test_pack_converts_pubkey_to_index() {
    let owner = Pubkey::new_unique();
    let record = PlaceholderRecord {
        compression_info: None,
        owner,
        name: "test placeholder".to_string(),
        placeholder_id: 1,
        counter: 100,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    // The owner should have been added to packed_accounts
    // and packed.owner should be the index (0 for first pubkey)
    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.placeholder_id, 1);
    assert_eq!(packed.counter, 100);

    let mut packed_accounts = PackedAccounts::default();
    packed_accounts.insert_or_get(Pubkey::new_unique());
    let packed = record.pack(&mut packed_accounts).unwrap();

    // The owner should have been added to packed_accounts
    // and packed.owner should be the index (1 for second pubkey)
    assert_eq!(packed.owner, 1u8);
    assert_eq!(packed.placeholder_id, 1);
    assert_eq!(packed.counter, 100);
}

#[test]
fn test_pack_reuses_same_pubkey_index() {
    let owner = Pubkey::new_unique();

    let record1 = PlaceholderRecord {
        compression_info: None,
        owner,
        name: "placeholder1".to_string(),
        placeholder_id: 1,
        counter: 1,
    };

    let record2 = PlaceholderRecord {
        compression_info: None,
        owner,
        name: "placeholder2".to_string(),
        placeholder_id: 2,
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
    let record1 = PlaceholderRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        name: "placeholder1".to_string(),
        placeholder_id: 1,
        counter: 1,
    };

    let record2 = PlaceholderRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        name: "placeholder2".to_string(),
        placeholder_id: 2,
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
fn test_pack_sets_compression_info_to_none() {
    let record_with_info = PlaceholderRecord {
        compression_info: Some(CompressionInfo::default()),
        owner: Pubkey::new_unique(),
        name: "test".to_string(),
        placeholder_id: 1,
        counter: 100,
    };

    let record_without_info = PlaceholderRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        name: "test".to_string(),
        placeholder_id: 2,
        counter: 200,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record_with_info.pack(&mut packed_accounts).unwrap();
    let packed2 = record_without_info.pack(&mut packed_accounts).unwrap();

    // Both packed structs should have compression_info = None
    assert!(
        packed1.compression_info.is_none(),
        "pack should set compression_info to None (even if input has Some)"
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

    let record1 = PlaceholderRecord {
        compression_info: None,
        owner: owner1,
        name: "placeholder1".to_string(),
        placeholder_id: 1,
        counter: 1,
    };

    let record2 = PlaceholderRecord {
        compression_info: None,
        owner: owner2,
        name: "placeholder2".to_string(),
        placeholder_id: 2,
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
