//! D2 Tests: NoCompressAsRecord trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `NoCompressAsRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//! - CompressiblePack -> Pack + Unpack + PackedNoCompressAsRecord

use csdk_anchor_full_derived_test::{NoCompressAsRecord, PackedNoCompressAsRecord};
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

impl CompressibleTestFactory for NoCompressAsRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::default(),
            owner: Pubkey::new_unique(),
            counter: 0,
            flag: false,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::compressed(),
            owner: Pubkey::new_unique(),
            counter: 0,
            flag: false,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(NoCompressAsRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_preserves_all_fields() {
    let owner = Pubkey::new_unique();
    let counter = 123u64;
    let flag = true;

    let record = NoCompressAsRecord {
        compression_info: CompressionInfo::default(),
        owner,
        counter,
        flag,
    };

    let compressed = record.compress_as();

    // No compress_as attribute, all fields should be preserved
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.counter, counter);
    assert_eq!(compressed.flag, flag);
}

#[test]
fn test_compress_as_with_multiple_flag_values() {
    let owner = Pubkey::new_unique();
    let counter = 555u64;

    for flag_val in &[true, false] {
        let record = NoCompressAsRecord {
            compression_info: CompressionInfo::default(),
            owner,
            counter,
            flag: *flag_val,
        };

        let compressed = record.compress_as();
        assert_eq!(compressed.flag, *flag_val, "flag should be preserved");
        assert_eq!(compressed.counter, counter, "counter should be preserved");
    }
}

#[test]
fn test_compress_as_when_compression_info_already_none() {
    let owner = Pubkey::new_unique();
    let counter = 789u64;
    let flag = true;

    let record = NoCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        counter,
        flag,
    };

    let compressed = record.compress_as();

    // Should still work and preserve all fields    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.counter, counter);
    assert_eq!(compressed.flag, flag);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_counter() {
    let owner = Pubkey::new_unique();

    let record1 = NoCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        counter: 1,
        flag: false,
    };

    let record2 = NoCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
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

    let record1 = NoCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        counter: 100,
        flag: true,
    };

    let record2 = NoCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        counter: 100,
        flag: false,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(hash1, hash2, "different flag should produce different hash");
}

#[test]
fn test_hash_differs_for_different_owner() {
    let record1 = NoCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        counter: 100,
        flag: false,
    };

    let record2 = NoCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
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
    let packed = PackedNoCompressAsRecord {        owner: 0,
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
    let record = NoCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        counter: 100,
        flag: true,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.counter, 100);
    assert!(packed.flag);
}

#[test]
fn test_pack_reuses_same_pubkey_index() {
    let owner = Pubkey::new_unique();

    let record1 = NoCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        counter: 1,
        flag: true,
    };

    let record2 = NoCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        counter: 2,
        flag: false,
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
    let record1 = NoCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        counter: 1,
        flag: true,
    };

    let record2 = NoCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        counter: 2,
        flag: false,
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
fn test_pack_sets_compression_info_to_none() {
    let record_with_info = NoCompressAsRecord {
        compression_info: CompressionInfo::default(),
        owner: Pubkey::new_unique(),
        counter: 100,
        flag: true,
    };

    let record_without_info = NoCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        counter: 200,
        flag: false,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record_with_info.pack(&mut packed_accounts).unwrap();
    let packed2 = record_without_info.pack(&mut packed_accounts).unwrap();}

#[test]
fn test_pack_stores_pubkeys_in_packed_accounts() {
    let owner1 = Pubkey::new_unique();
    let owner2 = Pubkey::new_unique();

    let record1 = NoCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: owner1,
        counter: 1,
        flag: true,
    };

    let record2 = NoCompressAsRecord {
        compression_info: CompressionInfo::compressed(),
        owner: owner2,
        counter: 2,
        flag: false,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts).unwrap();
    let packed2 = record2.pack(&mut packed_accounts).unwrap();

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
        let record = NoCompressAsRecord {
            compression_info: CompressionInfo::compressed(),
            owner: *owner,
            counter: 0,
            flag: false,
        };
        let packed = record.pack(&mut packed_accounts).unwrap();
        indices.push(packed.owner);
    }

    assert_eq!(indices, vec![0, 1, 2, 3, 4], "indices should be sequential");
}
