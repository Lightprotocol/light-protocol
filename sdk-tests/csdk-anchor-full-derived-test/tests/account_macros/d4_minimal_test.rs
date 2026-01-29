//! D4 Tests: MinimalRecord trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `MinimalRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//!
//! MinimalRecord has NO Pubkey fields, so Pack/Unpack traits are NOT generated.

use csdk_anchor_full_derived_test::MinimalRecord;
use light_hasher::{DataHasher, Sha256};
use light_sdk::interface::{CompressAs, CompressionInfo};

use super::shared::CompressibleTestFactory;
use crate::generate_trait_tests;

// =============================================================================
// Factory Implementation
// =============================================================================

impl CompressibleTestFactory for MinimalRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::default(),
            value: 42u64,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::compressed(),
            value: 42u64,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(MinimalRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_preserves_value() {
    let value = 999u64;

    let record = MinimalRecord {
        compression_info: CompressionInfo::default(),
        value,
    };

    let compressed = record.compress_as();
    assert_eq!(compressed.value, value);
}

#[test]
fn test_compress_as_when_compression_info_already_none() {
    let value = 123u64;

    let record = MinimalRecord {
        compression_info: CompressionInfo::compressed(),
        value,
    };

    let _compressed = record.compress_as();

    // Should still work and preserve fields    assert_eq!(_compressed.value, value);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_value() {
    let record1 = MinimalRecord {
        compression_info: CompressionInfo::compressed(),
        value: 1,
    };

    let record2 = MinimalRecord {
        compression_info: CompressionInfo::compressed(),
        value: 2,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different value should produce different hash"
    );
}

#[test]
fn test_hash_same_for_same_value() {
    let value = 100u64;

    let record1 = MinimalRecord {
        compression_info: CompressionInfo::compressed(),
        value,
    };

    let record2 = MinimalRecord {
        compression_info: CompressionInfo::compressed(),
        value,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_eq!(hash1, hash2, "same value should produce same hash");
}
