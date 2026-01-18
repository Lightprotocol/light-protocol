//! D1 Tests: ArrayRecord trait derive tests
//!
//! Tests each trait derived by `RentFreeAccount` macro for `ArrayRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//! - CompressiblePack -> Pack + Unpack (identity implementation with array fields)
//!
//! Note: Since ArrayRecord has no Pubkey fields, the Pack trait generates an identity
//! implementation where Packed = Self. Array fields are directly copied in pack/unpack.
//! Therefore, no Pack/Unpack tests are needed.

use csdk_anchor_full_derived_test::ArrayRecord;
use light_hasher::{DataHasher, Sha256};
use light_sdk::compressible::{CompressAs, CompressionInfo};

use super::shared::CompressibleTestFactory;
use crate::generate_trait_tests;

// =============================================================================
// Factory Implementation
// =============================================================================

impl CompressibleTestFactory for ArrayRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: Some(CompressionInfo::default()),
            hash: [0u8; 32],
            short_data: [0u8; 8],
            counter: 0,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: None,
            hash: [0u8; 32],
            short_data: [0u8; 8],
            counter: 0,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(ArrayRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_preserves_other_fields() {
    let mut hash = [0u8; 32];
    hash[0] = 1;
    hash[31] = 255;

    let mut short_data = [0u8; 8];
    short_data[0] = 42;
    short_data[7] = 99;

    let counter = 999u64;

    let record = ArrayRecord {
        compression_info: Some(CompressionInfo::default()),
        hash,
        short_data,
        counter,
    };

    let compressed = record.compress_as();
    assert_eq!(compressed.hash, hash);
    assert_eq!(compressed.short_data, short_data);
    assert_eq!(compressed.counter, counter);
}

#[test]
fn test_compress_as_when_compression_info_already_none() {
    let mut hash = [0u8; 32];
    hash[15] = 128;

    let mut short_data = [0u8; 8];
    short_data[3] = 77;

    let counter = 123u64;

    let record = ArrayRecord {
        compression_info: None,
        hash,
        short_data,
        counter,
    };

    let compressed = record.compress_as();

    // Should still work and preserve fields
    assert!(compressed.compression_info.is_none());
    assert_eq!(compressed.hash, hash);
    assert_eq!(compressed.short_data, short_data);
    assert_eq!(compressed.counter, counter);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_counter() {
    let hash = [5u8; 32];
    let short_data = [10u8; 8];

    let record1 = ArrayRecord {
        compression_info: None,
        hash,
        short_data,
        counter: 1,
    };

    let record2 = ArrayRecord {
        compression_info: None,
        hash,
        short_data,
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
fn test_hash_differs_for_different_hash_array() {
    let mut hash1_array = [0u8; 32];
    hash1_array[0] = 1;

    let mut hash2_array = [0u8; 32];
    hash2_array[0] = 2;

    let short_data = [10u8; 8];

    let record1 = ArrayRecord {
        compression_info: None,
        hash: hash1_array,
        short_data,
        counter: 100,
    };

    let record2 = ArrayRecord {
        compression_info: None,
        hash: hash2_array,
        short_data,
        counter: 100,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different hash array should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_short_data_array() {
    let hash = [5u8; 32];

    let mut short_data1 = [0u8; 8];
    short_data1[0] = 1;

    let mut short_data2 = [0u8; 8];
    short_data2[0] = 2;

    let record1 = ArrayRecord {
        compression_info: None,
        hash,
        short_data: short_data1,
        counter: 100,
    };

    let record2 = ArrayRecord {
        compression_info: None,
        hash,
        short_data: short_data2,
        counter: 100,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different short_data array should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_array_position() {
    let short_data = [10u8; 8];

    let mut hash1_array = [0u8; 32];
    hash1_array[0] = 5;

    let mut hash2_array = [0u8; 32];
    hash2_array[31] = 5; // same value, different position

    let record1 = ArrayRecord {
        compression_info: None,
        hash: hash1_array,
        short_data,
        counter: 100,
    };

    let record2 = ArrayRecord {
        compression_info: None,
        hash: hash2_array,
        short_data,
        counter: 100,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different array positions should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_zero_vs_nonzero_array() {
    let zero_hash = [0u8; 32];
    let nonzero_hash = [1u8; 32];
    let short_data = [10u8; 8];

    let record1 = ArrayRecord {
        compression_info: None,
        hash: zero_hash,
        short_data,
        counter: 100,
    };

    let record2 = ArrayRecord {
        compression_info: None,
        hash: nonzero_hash,
        short_data,
        counter: 100,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "zero vs non-zero array should produce different hash"
    );
}
