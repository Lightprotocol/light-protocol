//! D4 Tests: AllCompositionRecord trait derive tests
//!
//! Tests each trait derived by `RentFreeAccount` macro for `AllCompositionRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//! - CompressiblePack -> Pack + Unpack + PackedAllCompositionRecord
//!
//! AllCompositionRecord has 3 Pubkey fields + 1 Option<Pubkey> field and uses
//! #[compress_as(cached_time = 0, end_time = None)] to override field values.
//! This tests full Pack/Unpack behavior with compress_as attribute overrides.

use csdk_anchor_full_derived_test::{AllCompositionRecord, PackedAllCompositionRecord};
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

impl CompressibleTestFactory for AllCompositionRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: Some(CompressionInfo::default()),
            owner: Pubkey::new_unique(),
            delegate: Pubkey::new_unique(),
            authority: Pubkey::new_unique(),
            close_authority: Some(Pubkey::new_unique()),
            name: "test".to_string(),
            hash: [0u8; 32],
            start_time: 100,
            cached_time: 200,
            end_time: Some(300),
            counter_1: 1,
            counter_2: 2,
            counter_3: 3,
            flag_1: true,
            flag_2: false,
            score: Some(50),
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: None,
            owner: Pubkey::new_unique(),
            delegate: Pubkey::new_unique(),
            authority: Pubkey::new_unique(),
            close_authority: Some(Pubkey::new_unique()),
            name: "test".to_string(),
            hash: [0u8; 32],
            start_time: 100,
            cached_time: 200,
            end_time: Some(300),
            counter_1: 1,
            counter_2: 2,
            counter_3: 3,
            flag_1: true,
            flag_2: false,
            score: Some(50),
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(AllCompositionRecord);

// =============================================================================
// Struct-Specific CompressAs Tests with Attribute Overrides
// =============================================================================

#[test]
fn test_compress_as_overrides_cached_time() {
    // #[compress_as(cached_time = 0, ...)] should set cached_time to 0
    let record = AllCompositionRecord {
        compression_info: Some(CompressionInfo::default()),
        owner: Pubkey::new_unique(),
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: Some(Pubkey::new_unique()),
        name: "test".to_string(),
        hash: [0u8; 32],
        start_time: 100,
        cached_time: 999, // This should be overridden to 0
        end_time: Some(300),
        counter_1: 1,
        counter_2: 2,
        counter_3: 3,
        flag_1: true,
        flag_2: false,
        score: Some(50),
    };

    let compressed = record.compress_as();

    // cached_time should be 0 due to #[compress_as(cached_time = 0)]
    assert_eq!(
        compressed.cached_time, 0,
        "compress_as should override cached_time to 0"
    );
}

#[test]
fn test_compress_as_overrides_end_time() {
    // #[compress_as(..., end_time = None)] should set end_time to None
    let record = AllCompositionRecord {
        compression_info: Some(CompressionInfo::default()),
        owner: Pubkey::new_unique(),
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: Some(Pubkey::new_unique()),
        name: "test".to_string(),
        hash: [0u8; 32],
        start_time: 100,
        cached_time: 200,
        end_time: Some(999), // This should be overridden to None
        counter_1: 1,
        counter_2: 2,
        counter_3: 3,
        flag_1: true,
        flag_2: false,
        score: Some(50),
    };

    let compressed = record.compress_as();

    // end_time should be None due to #[compress_as(..., end_time = None)]
    assert!(
        compressed.end_time.is_none(),
        "compress_as should override end_time to None"
    );
}

#[test]
fn test_compress_as_preserves_start_time() {
    // start_time is NOT in #[compress_as(...)], so it should NOT be overridden
    let start_time_value = 777u64;

    let record = AllCompositionRecord {
        compression_info: Some(CompressionInfo::default()),
        owner: Pubkey::new_unique(),
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: Some(Pubkey::new_unique()),
        name: "test".to_string(),
        hash: [0u8; 32],
        start_time: start_time_value,
        cached_time: 200,
        end_time: Some(300),
        counter_1: 1,
        counter_2: 2,
        counter_3: 3,
        flag_1: true,
        flag_2: false,
        score: Some(50),
    };

    let compressed = record.compress_as();

    // start_time should be preserved because it's not in the #[compress_as(...)]
    assert_eq!(
        compressed.start_time, start_time_value,
        "compress_as should NOT override start_time (not in compress_as attribute)"
    );
}

#[test]
fn test_compress_as_preserves_non_override_fields() {
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let record = AllCompositionRecord {
        compression_info: Some(CompressionInfo::default()),
        owner,
        delegate,
        authority,
        close_authority: Some(Pubkey::new_unique()),
        name: "custom_name".to_string(),
        hash: [5u8; 32],
        start_time: 500,
        cached_time: 600,
        end_time: Some(700),
        counter_1: 11,
        counter_2: 22,
        counter_3: 33,
        flag_1: false,
        flag_2: true,
        score: Some(99),
    };

    let compressed = record.compress_as();

    // Fields not in compress_as should be preserved
    assert_eq!(compressed.owner, owner);
    assert_eq!(compressed.delegate, delegate);
    assert_eq!(compressed.authority, authority);
    assert_eq!(compressed.counter_1, 11);
    assert_eq!(compressed.counter_2, 22);
    assert_eq!(compressed.counter_3, 33);
    assert!(!compressed.flag_1);
    assert!(compressed.flag_2);
    assert_eq!(compressed.score, Some(99));
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_owner() {
    let record1 = AllCompositionRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: None,
        name: "test".to_string(),
        hash: [0u8; 32],
        start_time: 100,
        cached_time: 200,
        end_time: None,
        counter_1: 1,
        counter_2: 2,
        counter_3: 3,
        flag_1: true,
        flag_2: false,
        score: None,
    };

    let record2 = AllCompositionRecord {
        compression_info: None,
        owner: Pubkey::new_unique(),
        delegate: record1.delegate,
        authority: record1.authority,
        close_authority: None,
        name: "test".to_string(),
        hash: [0u8; 32],
        start_time: 100,
        cached_time: 200,
        end_time: None,
        counter_1: 1,
        counter_2: 2,
        counter_3: 3,
        flag_1: true,
        flag_2: false,
        score: None,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different owner should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_counter_3() {
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let record1 = AllCompositionRecord {
        compression_info: None,
        owner,
        delegate,
        authority,
        close_authority: None,
        name: "test".to_string(),
        hash: [0u8; 32],
        start_time: 100,
        cached_time: 200,
        end_time: None,
        counter_1: 1,
        counter_2: 2,
        counter_3: 3,
        flag_1: true,
        flag_2: false,
        score: None,
    };

    let mut record2 = record1.clone();
    record2.counter_3 = 999;

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different counter_3 should produce different hash"
    );
}

// =============================================================================
// Pack/Unpack Tests (struct-specific, cannot be generic)
// =============================================================================

#[test]
fn test_packed_struct_has_u8_pubkey_fields() {
    // Verify PackedAllCompositionRecord has direct Pubkey fields as u8
    // Note: Option<Pubkey> is NOT converted to Option<u8> - it stays as Option<Pubkey>
    let close_authority = Pubkey::new_unique();
    let packed = PackedAllCompositionRecord {
        owner: 0,
        delegate: 1,
        authority: 2,
        close_authority: Some(close_authority),
        compression_info: None,
        name: "test".to_string(),
        hash: [0u8; 32],
        start_time: 100,
        cached_time: 0, // overridden by compress_as
        end_time: None, // overridden by compress_as
        counter_1: 1,
        counter_2: 2,
        counter_3: 3,
        flag_1: true,
        flag_2: false,
        score: Some(50),
    };

    assert_eq!(packed.owner, 0u8);
    assert_eq!(packed.delegate, 1u8);
    assert_eq!(packed.authority, 2u8);
    assert_eq!(packed.close_authority, Some(close_authority));
}

#[test]
fn test_pack_converts_all_pubkeys_to_indices() {
    let owner = Pubkey::new_unique();
    let delegate = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let close_authority = Pubkey::new_unique();

    let record = AllCompositionRecord {
        owner,
        delegate,
        authority,
        close_authority: Some(close_authority),
        compression_info: None,
        name: "test".to_string(),
        hash: [0u8; 32],
        start_time: 100,
        cached_time: 200,
        end_time: Some(300),
        counter_1: 1,
        counter_2: 2,
        counter_3: 3,
        flag_1: true,
        flag_2: false,
        score: Some(50),
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts);

    // Direct Pubkey fields are converted to u8 indices
    assert_eq!(packed.owner, 0u8); // First pubkey
    assert_eq!(packed.delegate, 1u8); // Second pubkey
    assert_eq!(packed.authority, 2u8); // Third pubkey
                                       // Option<Pubkey> is NOT converted to Option<u8> - it stays as Option<Pubkey>
    assert_eq!(packed.close_authority, Some(close_authority));
}

#[test]
fn test_pack_does_not_apply_compress_as_overrides() {
    // Note: Pack does NOT apply compress_as overrides. Those are only applied
    // by the CompressAs trait's compress_as() method. If you need overrides
    // applied, call compress_as() first, then pack() the result.
    let close_authority = Pubkey::new_unique();
    let record = AllCompositionRecord {
        owner: Pubkey::new_unique(),
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: Some(close_authority),
        compression_info: Some(CompressionInfo::default()),
        name: "test".to_string(),
        hash: [0u8; 32],
        start_time: 100,
        cached_time: 999,
        end_time: Some(999),
        counter_1: 1,
        counter_2: 2,
        counter_3: 3,
        flag_1: true,
        flag_2: false,
        score: Some(50),
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts);

    // Pack preserves field values - compress_as overrides are NOT applied
    assert_eq!(packed.cached_time, 999, "pack preserves cached_time value");
    assert_eq!(packed.end_time, Some(999), "pack preserves end_time value");
    // Option<Pubkey> stays as Option<Pubkey>
    assert_eq!(packed.close_authority, Some(close_authority));
}

#[test]
fn test_compress_as_then_pack_applies_overrides() {
    // The correct way to pack with compress_as overrides:
    // call compress_as() first, then pack() the result
    let close_authority = Pubkey::new_unique();
    let record = AllCompositionRecord {
        owner: Pubkey::new_unique(),
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: Some(close_authority),
        compression_info: Some(CompressionInfo::default()),
        name: "test".to_string(),
        hash: [0u8; 32],
        start_time: 100,
        cached_time: 999,    // Should become 0 after compress_as
        end_time: Some(999), // Should become None after compress_as
        counter_1: 1,
        counter_2: 2,
        counter_3: 3,
        flag_1: true,
        flag_2: false,
        score: Some(50),
    };

    // Chain compress_as() then pack()
    let compressed = record.compress_as();
    let mut packed_accounts = PackedAccounts::default();
    let packed = compressed.pack(&mut packed_accounts);

    // compress_as overrides ARE applied when chained
    assert_eq!(
        packed.cached_time, 0,
        "compress_as().pack() applies cached_time = 0 override"
    );
    assert!(
        packed.end_time.is_none(),
        "compress_as().pack() applies end_time = None override"
    );
    // Non-overridden fields preserved
    assert_eq!(packed.start_time, 100);
    assert_eq!(packed.counter_1, 1);
}

#[test]
fn test_pack_preserves_start_time_without_override() {
    let start_time_value = 555u64;

    let record = AllCompositionRecord {
        owner: Pubkey::new_unique(),
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: None,
        compression_info: None,
        name: "test".to_string(),
        hash: [0u8; 32],
        start_time: start_time_value,
        cached_time: 200,
        end_time: None,
        counter_1: 1,
        counter_2: 2,
        counter_3: 3,
        flag_1: true,
        flag_2: false,
        score: None,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts);

    assert_eq!(
        packed.start_time, start_time_value,
        "pack should preserve start_time (not in compress_as override)"
    );
}

#[test]
fn test_pack_reuses_duplicate_pubkeys_for_direct_fields() {
    // Test that same Pubkey used in multiple direct Pubkey fields gets same index
    let shared_pubkey = Pubkey::new_unique();

    let record1 = AllCompositionRecord {
        owner: shared_pubkey,
        delegate: shared_pubkey, // Same as owner
        authority: Pubkey::new_unique(),
        close_authority: Some(shared_pubkey), // Option<Pubkey> is NOT packed
        compression_info: None,
        name: "test".to_string(),
        hash: [0u8; 32],
        start_time: 100,
        cached_time: 200,
        end_time: None,
        counter_1: 1,
        counter_2: 2,
        counter_3: 3,
        flag_1: true,
        flag_2: false,
        score: None,
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record1.pack(&mut packed_accounts);

    // owner and delegate are the same pubkey, should get the same index
    assert_eq!(
        packed.owner, packed.delegate,
        "same pubkey should get same index"
    );

    // Option<Pubkey> is NOT converted to Option<u8> - it stays as Option<Pubkey>
    assert_eq!(packed.close_authority, Some(shared_pubkey));

    // Only 2 unique pubkeys stored (shared_pubkey and authority)
    let stored_pubkeys = packed_accounts.packed_pubkeys();
    assert_eq!(stored_pubkeys.len(), 2, "should have 2 unique pubkeys");
}

#[test]
fn test_pack_sets_compression_info_to_none() {
    let record = AllCompositionRecord {
        owner: Pubkey::new_unique(),
        delegate: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        close_authority: Some(Pubkey::new_unique()),
        compression_info: Some(CompressionInfo::default()),
        name: "test".to_string(),
        hash: [0u8; 32],
        start_time: 100,
        cached_time: 200,
        end_time: Some(300),
        counter_1: 1,
        counter_2: 2,
        counter_3: 3,
        flag_1: true,
        flag_2: false,
        score: Some(50),
    };

    let mut packed_accounts = PackedAccounts::default();
    let packed = record.pack(&mut packed_accounts);

    assert!(
        packed.compression_info.is_none(),
        "pack should set compression_info to None"
    );
}
