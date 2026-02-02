//! D1 Tests: AllFieldTypesRecord trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `AllFieldTypesRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//!
//! Comprehensive test exercising all field type code paths:
//! - Multiple Pubkeys (owner, delegate, authority) -> u8 indices
//! - Option<Pubkey> (close_authority) -> remains Option<Pubkey> (NOT converted to u8)
//! - String (name) -> clone() path
//! - Arrays (hash) -> direct copy
//! - Option<primitives> (end_time, enabled) -> unchanged
//! - Regular primitives (counter, flag) -> direct copy

use csdk_anchor_full_derived_test::{AllFieldTypesRecord, PackedAllFieldTypesRecord};
use light_hasher::{DataHasher, Sha256};
use light_sdk::{
    compressible::{CompressAs, CompressionInfo, CompressionState, Pack},
    instruction::PackedAccounts,
};
use solana_pubkey::Pubkey;

use super::shared::CompressibleTestFactory;
use crate::generate_trait_tests;

// =============================================================================
// Factory Implementation
// =============================================================================

impl CompressibleTestFactory for AllFieldTypesRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::default(),
            owner: Pubkey::new_unique(),
            delegate: Pubkey::new_unique(),
            authority: Pubkey::new_unique(),
            close_authority: Some(Pubkey::new_unique()),
            name: "test name".to_string(),
            hash: [0u8; 32],
            end_time: Some(1000),
            enabled: Some(true),
            counter: 0,
            flag: false,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::compressed(),
            owner: Pubkey::new_unique(),
            delegate: Pubkey::new_unique(),
            authority: Pubkey::new_unique(),
            close_authority: None,
            name: "test name".to_string(),
            hash: [0u8; 32],
            end_time: None,
            enabled: None,
            counter: 0,
            flag: false,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(AllFieldTypesRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_preserves_all_field_types() {
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let close_authority = Some(Pubkey::new_unique());
    let name = "Alice".to_string();
    let mut hash = [0u8; 32];
    hash[0] = 42;
    let end_time = Some(5000u64);
    let enabled = Some(false);
    let counter = 999u64;
    let flag = true;

    let record = AllFieldTypesRecord {
        compression_info: CompressionInfo::default(),
        owner,
        delegate,
        authority,
        close_authority,
        name: name.clone(),
        hash,
        end_time,
        enabled,
        counter,
        flag,
    };

    let compressed = record.compress_as();
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.delegate, delegate);
    assert_eq!(compressed.authority, authority);
    assert_eq!(compressed.close_authority, close_authority);
    assert_eq!(compressed.name, name);
    assert_eq!(compressed.hash, hash);
    assert_eq!(compressed.end_time, end_time);
    assert_eq!(compressed.enabled, enabled);
    assert_eq!(compressed.counter, counter);
    assert_eq!(compressed.flag, flag);
}

#[test]
fn test_compress_as_when_compression_info_already_compressed() {
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let name = "Bob".to_string();
    let counter = 123u64;

    let record = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        authority,
        close_authority: None,
        name: name.clone(),
        hash: [0u8; 32],
        end_time: None,
        enabled: None,
        counter,
        flag: false,
    };

    let compressed = record.compress_as();

    // Should still work and preserve all fields
    assert_eq!(
        compressed.compression_info.state,
        CompressionState::Compressed
    );
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.counter, counter);
    assert_eq!(compressed.name, name);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_pubkey_field() {
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let record1 = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        delegate,
        authority,
        close_authority: None,
        name: "test".to_string(),
        hash: [0u8; 32],
        end_time: None,
        enabled: None,
        counter: 100,
        flag: false,
    };

    let record2 = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::new_unique(),
        delegate,
        authority,
        close_authority: None,
        name: "test".to_string(),
        hash: [0u8; 32],
        end_time: None,
        enabled: None,
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

#[test]
fn test_hash_differs_for_different_option_pubkey_field() {
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let record1 = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        authority,
        close_authority: Some(Pubkey::new_unique()),
        name: "test".to_string(),
        hash: [0u8; 32],
        end_time: None,
        enabled: None,
        counter: 100,
        flag: false,
    };

    let record2 = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        authority,
        close_authority: None,
        name: "test".to_string(),
        hash: [0u8; 32],
        end_time: None,
        enabled: None,
        counter: 100,
        flag: false,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different close_authority (Some vs None) should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_string_field() {
    let owner = Pubkey::new_unique();

    let record1 = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: None,
        name: "Alice".to_string(),
        hash: [0u8; 32],
        end_time: None,
        enabled: None,
        counter: 100,
        flag: false,
    };

    let record2 = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: None,
        name: "Bob".to_string(),
        hash: [0u8; 32],
        end_time: None,
        enabled: None,
        counter: 100,
        flag: false,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(hash1, hash2, "different name should produce different hash");
}

#[test]
fn test_hash_differs_for_different_array_field() {
    let owner = Pubkey::new_unique();
    let mut hash1_array = [0u8; 32];
    hash1_array[0] = 1;

    let mut hash2_array = [0u8; 32];
    hash2_array[0] = 2;

    let record1 = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: None,
        name: "test".to_string(),
        hash: hash1_array,
        end_time: None,
        enabled: None,
        counter: 100,
        flag: false,
    };

    let record2 = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: None,
        name: "test".to_string(),
        hash: hash2_array,
        end_time: None,
        enabled: None,
        counter: 100,
        flag: false,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different hash array should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_option_primitive() {
    let owner = Pubkey::new_unique();

    let record1 = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: None,
        name: "test".to_string(),
        hash: [0u8; 32],
        end_time: Some(1000),
        enabled: None,
        counter: 100,
        flag: false,
    };

    let record2 = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: None,
        name: "test".to_string(),
        hash: [0u8; 32],
        end_time: Some(2000),
        enabled: None,
        counter: 100,
        flag: false,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different end_time should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_primitive() {
    let owner = Pubkey::new_unique();

    let record1 = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: None,
        name: "test".to_string(),
        hash: [0u8; 32],
        end_time: None,
        enabled: None,
        counter: 100,
        flag: false,
    };

    let record2 = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: None,
        name: "test".to_string(),
        hash: [0u8; 32],
        end_time: None,
        enabled: None,
        counter: 200,
        flag: false,
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
fn test_packed_struct_has_all_types_converted() {
    // Verify PackedAllFieldTypesRecord has the correct field types
    // Note: Option<Pubkey> is NOT converted to Option<u8> - it stays as Option<Pubkey>
    let close_authority = Pubkey::new_unique();
    let packed = PackedAllFieldTypesRecord {
        owner: 0,
        delegate: 1,
        authority: 2,
        close_authority: Some(close_authority),
        name: "test".to_string(),
        hash: [0u8; 32],
        end_time: Some(1000),
        enabled: Some(true),
        counter: 42,
        flag: false,
    };

    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.delegate, 1u8);
    assert_eq!(packed.authority, 2u8);
    assert_eq!(packed.close_authority, Some(close_authority));
    assert_eq!(packed.name, "test".to_string());
    assert_eq!(packed.counter, 42u64);
    assert!(!packed.flag);
}

#[test]
fn test_pack_converts_all_pubkey_types() {
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let close_authority = Pubkey::new_unique();
    let name = "test".to_string();

    let record = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        authority,
        close_authority: Some(close_authority),
        name: name.clone(),
        hash: [0u8; 32],
        end_time: Some(1000),
        enabled: Some(true),
        counter: 100,
        flag: true,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    // Direct Pubkey fields are converted to u8 indices
    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.delegate, 1u8);
    assert_eq!(packed.authority, 2u8);
    // Option<Pubkey> is NOT converted to Option<u8> - it stays as Option<Pubkey>
    assert_eq!(packed.close_authority, Some(close_authority));
    assert_eq!(packed.name, name);
    assert_eq!(packed.counter, 100);
    assert!(packed.flag);

    // Only direct Pubkey fields are stored in packed_accounts (not Option<Pubkey>)
    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 3);
    assert_eq!(stored_pubkeys[0], owner.to_bytes());
    assert_eq!(stored_pubkeys[1], delegate.to_bytes());
    assert_eq!(stored_pubkeys[2], authority.to_bytes());
}

#[test]
fn test_pack_with_option_pubkey_none() {
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let record = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        authority,
        close_authority: None,
        name: "test".to_string(),
        hash: [0u8; 32],
        end_time: None,
        enabled: None,
        counter: 100,
        flag: false,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    // Only three pubkeys should have been added
    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.delegate, 1u8);
    assert_eq!(packed.authority, 2u8);
    assert_eq!(
        packed.close_authority, None,
        "Option::None should remain None"
    );

    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 3);
}

#[test]
fn test_pack_reuses_pubkey_indices() {
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let record1 = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        authority,
        close_authority: None,
        name: "test1".to_string(),
        hash: [0u8; 32],
        end_time: None,
        enabled: None,
        counter: 1,
        flag: false,
    };

    let record2 = AllFieldTypesRecord {
        compression_info: CompressionInfo::compressed(),
        owner,
        delegate,
        authority,
        close_authority: None,
        name: "test2".to_string(),
        hash: [0u8; 32],
        end_time: None,
        enabled: None,
        counter: 2,
        flag: true,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed1 = record1.pack(&mut packed_accounts).unwrap();
    let packed2 = record2.pack(&mut packed_accounts).unwrap();

    // Same pubkeys should get same indices
    assert_eq!(packed1.owner, packed2.owner);
    assert_eq!(packed1.delegate, packed2.delegate);
    assert_eq!(packed1.authority, packed2.authority);

    // Should still only have 3 pubkeys total
    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 3);
}

#[test]
fn test_pack_preserves_non_pubkey_fields() {
    let name = "AllFieldsTest".to_string();
    let mut hash = [0u8; 32];
    hash[0] = 99;
    let end_time = Some(9999u64);
    let enabled = Some(true);
    let counter = 12345u64;
    let flag = true;

    let record = AllFieldTypesRecord {
        compression_info: CompressionInfo::default(),
        owner: Pubkey::new_unique(),
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: None,
        name: name.clone(),
        hash,
        end_time,
        enabled,
        counter,
        flag,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts).unwrap();

    // All non-Pubkey fields should be preserved
    assert_eq!(packed.name, name);
    assert_eq!(packed.hash, hash);
    assert_eq!(packed.end_time, end_time);
    assert_eq!(packed.enabled, enabled);
    assert_eq!(packed.counter, counter);
    assert_eq!(packed.flag, flag);
}
