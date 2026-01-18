//! D4 Tests: LargeRecord trait derive tests
//!
//! Tests each trait derived by `RentFreeAccount` macro for `LargeRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//!
//! LargeRecord has NO Pubkey fields and 12 u64 fields (13 total including compression_info).
//! This exercises the SHA256 hash mode for large structs.
//! Pack/Unpack traits are NOT generated because there are no Pubkey fields.

use super::shared::CompressibleTestFactory;
use crate::generate_trait_tests;
use csdk_anchor_full_derived_test::LargeRecord;
use light_hasher::{DataHasher, Sha256};
use light_sdk::compressible::{CompressAs, CompressionInfo};

// =============================================================================
// Factory Implementation
// =============================================================================

impl CompressibleTestFactory for LargeRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: Some(CompressionInfo::default()),
            field_01: 1,
            field_02: 2,
            field_03: 3,
            field_04: 4,
            field_05: 5,
            field_06: 6,
            field_07: 7,
            field_08: 8,
            field_09: 9,
            field_10: 10,
            field_11: 11,
            field_12: 12,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: None,
            field_01: 1,
            field_02: 2,
            field_03: 3,
            field_04: 4,
            field_05: 5,
            field_06: 6,
            field_07: 7,
            field_08: 8,
            field_09: 9,
            field_10: 10,
            field_11: 11,
            field_12: 12,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(LargeRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_preserves_all_fields() {
    let record = LargeRecord {
        compression_info: Some(CompressionInfo::default()),
        field_01: 100,
        field_02: 200,
        field_03: 300,
        field_04: 400,
        field_05: 500,
        field_06: 600,
        field_07: 700,
        field_08: 800,
        field_09: 900,
        field_10: 1000,
        field_11: 1100,
        field_12: 1200,
    };

    let compressed = record.compress_as();

    // Verify all fields are preserved
    assert_eq!(compressed.field_01, 100);
    assert_eq!(compressed.field_02, 200);
    assert_eq!(compressed.field_03, 300);
    assert_eq!(compressed.field_04, 400);
    assert_eq!(compressed.field_05, 500);
    assert_eq!(compressed.field_06, 600);
    assert_eq!(compressed.field_07, 700);
    assert_eq!(compressed.field_08, 800);
    assert_eq!(compressed.field_09, 900);
    assert_eq!(compressed.field_10, 1000);
    assert_eq!(compressed.field_11, 1100);
    assert_eq!(compressed.field_12, 1200);
}

#[test]
fn test_compress_as_when_compression_info_already_none() {
    let record = LargeRecord {
        compression_info: None,
        field_01: 1,
        field_02: 2,
        field_03: 3,
        field_04: 4,
        field_05: 5,
        field_06: 6,
        field_07: 7,
        field_08: 8,
        field_09: 9,
        field_10: 10,
        field_11: 11,
        field_12: 12,
    };

    let compressed = record.compress_as();

    // Should still work and preserve all fields
    assert!(compressed.compression_info.is_none());
    assert_eq!(compressed.field_01, 1);
    assert_eq!(compressed.field_12, 12);
}

// =============================================================================
// Struct-Specific DataHasher Tests (SHA256 mode)
// =============================================================================

#[test]
fn test_hash_produces_32_bytes_for_large_struct() {
    let record = LargeRecord::without_compression_info();
    let hash = record.hash::<Sha256>().expect("hash should succeed");
    assert_eq!(hash.len(), 32, "SHA256 hash should produce 32 bytes");
}

#[test]
fn test_hash_differs_for_different_field_01() {
    let mut record1 = LargeRecord::without_compression_info();
    let mut record2 = LargeRecord::without_compression_info();

    record1.field_01 = 100;
    record2.field_01 = 200;

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different field_01 should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_field_06() {
    let mut record1 = LargeRecord::without_compression_info();
    let mut record2 = LargeRecord::without_compression_info();

    record1.field_06 = 600;
    record2.field_06 = 700;

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different field_06 should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_field_12() {
    let mut record1 = LargeRecord::without_compression_info();
    let mut record2 = LargeRecord::without_compression_info();

    record1.field_12 = 1200;
    record2.field_12 = 1300;

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different field_12 should produce different hash"
    );
}

#[test]
fn test_hash_same_for_same_large_struct() {
    let record1 = LargeRecord::without_compression_info();
    let record2 = record1.clone();

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_eq!(hash1, hash2, "identical large records should produce same hash");
}

#[test]
fn test_hash_includes_all_fields_by_changing_middle_field() {
    let mut record1 = LargeRecord::without_compression_info();
    let mut record2 = LargeRecord::without_compression_info();

    // Change a field in the middle
    record1.field_06 = 600;
    record2.field_06 = 999;

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "changing middle field should change hash (all fields included)"
    );
}
