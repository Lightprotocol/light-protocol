//! D1 Tests: OptionPrimitiveRecord trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `OptionPrimitiveRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//!
//! Note: Since OptionPrimitiveRecord has no Pubkey fields, the Pack trait generates an identity
//! implementation where Packed = Self. Option<primitive> types remain unchanged in the packed
//! struct (not converted to Option<u8>). Therefore, no Pack/Unpack tests are needed.

use csdk_anchor_full_derived_test::OptionPrimitiveRecord;
use light_hasher::{DataHasher, Sha256};
use light_sdk::interface::{CompressAs, CompressionInfo};

use super::shared::CompressibleTestFactory;
use crate::generate_trait_tests;

// =============================================================================
// Factory Implementation
// =============================================================================

impl CompressibleTestFactory for OptionPrimitiveRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::default(),
            counter: 0,
            end_time: Some(1000),
            enabled: Some(true),
            score: Some(50),
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::compressed(),
            counter: 0,
            end_time: None,
            enabled: None,
            score: None,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(OptionPrimitiveRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_preserves_other_fields() {
    let counter = 999u64;
    let end_time = Some(2000u64);
    let enabled = Some(false);
    let score = Some(100u32);

    let record = OptionPrimitiveRecord {
        compression_info: CompressionInfo::default(),
        counter,
        end_time,
        enabled,
        score,
    };

    let compressed = record.compress_as();
    assert_eq!(compressed.counter, counter);
    assert_eq!(compressed.end_time, end_time);
    assert_eq!(compressed.enabled, enabled);
    assert_eq!(compressed.score, score);
}

#[test]
fn test_compress_as_when_compression_info_already_none() {
    let counter = 123u64;
    let end_time = None;
    let enabled = Some(true);
    let score = None;

    let record = OptionPrimitiveRecord {
        compression_info: CompressionInfo::compressed(),
        counter,
        end_time,
        enabled,
        score,
    };

    let compressed = record.compress_as();

    // Should still work and preserve fields    assert_eq!(compressed.counter, counter);
    assert_eq!(compressed.end_time, end_time);
    assert_eq!(compressed.enabled, enabled);
    assert_eq!(compressed.score, score);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_counter() {
    let record1 = OptionPrimitiveRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 1,
        end_time: Some(1000),
        enabled: Some(true),
        score: Some(50),
    };

    let record2 = OptionPrimitiveRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 2,
        end_time: Some(1000),
        enabled: Some(true),
        score: Some(50),
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different counter should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_end_time() {
    let record1 = OptionPrimitiveRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 100,
        end_time: Some(1000),
        enabled: Some(true),
        score: Some(50),
    };

    let record2 = OptionPrimitiveRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 100,
        end_time: Some(2000),
        enabled: Some(true),
        score: Some(50),
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different end_time should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_enabled() {
    let record1 = OptionPrimitiveRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 100,
        end_time: Some(1000),
        enabled: Some(true),
        score: Some(50),
    };

    let record2 = OptionPrimitiveRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 100,
        end_time: Some(1000),
        enabled: Some(false),
        score: Some(50),
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different enabled should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_score() {
    let record1 = OptionPrimitiveRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 100,
        end_time: Some(1000),
        enabled: Some(true),
        score: Some(50),
    };

    let record2 = OptionPrimitiveRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 100,
        end_time: Some(1000),
        enabled: Some(true),
        score: Some(100),
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different score should produce different hash"
    );
}

#[test]
fn test_hash_differs_when_option_is_none_vs_some() {
    let record1 = OptionPrimitiveRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 100,
        end_time: None,
        enabled: Some(true),
        score: Some(50),
    };

    let record2 = OptionPrimitiveRecord {
        compression_info: CompressionInfo::compressed(),
        counter: 100,
        end_time: Some(1000),
        enabled: Some(true),
        score: Some(50),
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "Option None vs Some should produce different hash"
    );
}
