//! D1 Tests: NoPubkeyRecord trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `NoPubkeyRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//! - CompressiblePack -> Pack + Unpack (identity implementation: PackedNoPubkeyRecord = NoPubkeyRecord)
//!
//! Note: Since NoPubkeyRecord has no Pubkey fields, the Pack trait generates an identity
//! implementation where Packed = Self. Therefore, no Pack/Unpack tests are needed - the
//! struct is packed as-is without transformation.

use csdk_anchor_full_derived_test::NoPubkeyRecord;
use light_hasher::{DataHasher, Sha256};
use light_sdk::interface::{CompressAs, CompressionInfo};
use light_sdk::compressible::CompressionState;

use super::shared::CompressibleTestFactory;
use crate::generate_trait_tests;

// =============================================================================
// Factory Implementation
// =============================================================================

impl CompressibleTestFactory for NoPubkeyRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::default(),
            counter: 0,
            flag: false,
            value: 0,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::compressed(),
            counter: 0,
            flag: false,
            value: 0,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(NoPubkeyRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_preserves_other_fields() {
    let counter = 999u64;
    let flag = true;
    let value = 42u32;

    let record = NoPubkeyRecord {
        compression_info: CompressionInfo::default(),
        counter,
        flag,
        value,
    };

    let compressed = record.compress_as();
    assert_eq!(compressed.counter, counter);
    assert_eq!(compressed.flag, flag);
    assert_eq!(compressed.value, value);
}

#[test]
fn test_compress_as_when_compression_info_already_compressed() {
    let counter = 123u64;
    let flag = false;
    let value = 789u32;

    let record = NoPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        counter,
        flag,
        value,
    };

    let compressed = record.compress_as();

    // Should still work and preserve fields
    assert_eq!(compressed.compression_info.state, CompressionState::Compressed);
    assert_eq!(compressed.counter, counter);
    assert_eq!(compressed.flag, flag);
    assert_eq!(compressed.value, value);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_counter() {
    let record1 = NoPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 1,
        flag: true,
        value: 100,
    };

    let record2 = NoPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 2,
        flag: true,
        value: 100,
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
    let record1 = NoPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 100,
        flag: true,
        value: 50,
    };

    let record2 = NoPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 100,
        flag: false,
        value: 50,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(hash1, hash2, "different flag should produce different hash");
}

#[test]
fn test_hash_differs_for_different_value() {
    let record1 = NoPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 100,
        flag: true,
        value: 1,
    };

    let record2 = NoPubkeyRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 100,
        flag: true,
        value: 2,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different value should produce different hash"
    );
}
