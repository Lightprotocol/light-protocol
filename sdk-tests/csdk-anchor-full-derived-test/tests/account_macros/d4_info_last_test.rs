//! D4 Tests: InfoLastRecord trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `InfoLastRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//!
//! InfoLastRecord has 1 Pubkey field (owner) and demonstrates that
//! compression_info can be placed in non-first position (ordering test).

use csdk_anchor_full_derived_test::{InfoLastRecord, PackedInfoLastRecord};
use light_hasher::{DataHasher, Sha256};
use light_account::{CompressAs, CompressionInfo, CompressionState, Pack};
use light_sdk::instruction::PackedAccounts;
use solana_pubkey::Pubkey;

use super::shared::CompressibleTestFactory;
use crate::generate_trait_tests;

// =============================================================================
// Factory Implementation
// =============================================================================

impl CompressibleTestFactory for InfoLastRecord {
    fn with_compression_info() -> Self {
        Self {
            owner: Pubkey::new_unique(),
            counter: 0,
            flag: false,
            compression_info: CompressionInfo::default(),
        }
    }

    fn without_compression_info() -> Self {
        Self {
            owner: Pubkey::new_unique(),
            counter: 0,
            flag: false,
            compression_info: CompressionInfo::compressed(),
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(InfoLastRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_preserves_other_fields() {
    let owner = Pubkey::new_unique();
    let counter = 999u64;
    let flag = true;

    let record = InfoLastRecord {
        owner,
        counter,
        flag,
        compression_info: CompressionInfo::default(),
    };

    let compressed = record.compress_as();
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.counter, counter);
    assert_eq!(compressed.flag, flag);
}

#[test]
fn test_compress_as_preserves_all_field_types() {
    let owner = Pubkey::new_unique();

    let record = InfoLastRecord {
        owner,
        counter: 42,
        flag: true,
        compression_info: CompressionInfo::default(),
    };

    let compressed = record.compress_as();

    // Verify all fields are preserved despite compression_info being last
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.counter, 42);
    assert!(compressed.flag);
    assert!(compressed.compression_info.state == CompressionState::Compressed);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_counter() {
    let owner = Pubkey::new_unique();

    let record1 = InfoLastRecord {
        owner,
        counter: 1,
        flag: false,
        compression_info: CompressionInfo::compressed(),
    };

    let record2 = InfoLastRecord {
        owner,
        counter: 2,
        flag: false,
        compression_info: CompressionInfo::compressed(),
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
    let record1 = InfoLastRecord {
        owner: Pubkey::new_unique(),
        counter: 100,
        flag: false,
        compression_info: CompressionInfo::compressed(),
    };

    let record2 = InfoLastRecord {
        owner: Pubkey::new_unique(),
        counter: 100,
        flag: false,
        compression_info: CompressionInfo::compressed(),
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different owner should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_flag() {
    let owner = Pubkey::new_unique();

    let record1 = InfoLastRecord {
        owner,
        counter: 50,
        flag: true,
        compression_info: CompressionInfo::compressed(),
    };

    let record2 = InfoLastRecord {
        owner,
        counter: 50,
        flag: false,
        compression_info: CompressionInfo::compressed(),
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(hash1, hash2, "different flag should produce different hash");
}

// =============================================================================
// Pack/Unpack Tests (struct-specific, cannot be generic)
// =============================================================================

#[test]
fn test_packed_struct_has_u8_owner() {
    // Verify PackedInfoLastRecord has the expected structure
    // The Packed struct uses the same field name but changes type to u8
    let packed = PackedInfoLastRecord {
        owner: 0,
        counter: 42,
        flag: true,
    };

    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.counter, 42u64);
    assert!(packed.flag);
}

#[test]
fn test_pack_converts_pubkey_to_index() {
    let owner = Pubkey::new_unique();
    let record = InfoLastRecord {
        owner,
        counter: 100,
        flag: false,
        compression_info: CompressionInfo::compressed(),
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    // The owner should have been added to packed_accounts
    // and packed.owner should be the index (0 for first pubkey)
    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.counter, 100);
    assert!(!packed.flag);

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

    let record1 = InfoLastRecord {
        owner,
        counter: 1,
        flag: true,
        compression_info: CompressionInfo::compressed(),
    };

    let record2 = InfoLastRecord {
        owner,
        counter: 2,
        flag: false,
        compression_info: CompressionInfo::compressed(),
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
fn test_pack_preserves_counter_and_flag() {
    let owner = Pubkey::new_unique();
    let counter = 777u64;
    let flag = true;

    let record = InfoLastRecord {
        owner,
        counter,
        flag,
        compression_info: CompressionInfo::compressed(),
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    // counter and flag should be preserved
    assert_eq!(packed.counter, counter);
    assert_eq!(packed.flag, flag);
}

#[test]
fn test_pack_different_pubkeys_get_different_indices() {
    let record1 = InfoLastRecord {
        owner: Pubkey::new_unique(),
        counter: 1,
        flag: true,
        compression_info: CompressionInfo::compressed(),
    };

    let record2 = InfoLastRecord {
        owner: Pubkey::new_unique(),
        counter: 2,
        flag: false,
        compression_info: CompressionInfo::compressed(),
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
