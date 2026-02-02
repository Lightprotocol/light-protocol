//! D1 Tests: NonCopyRecord trait derive tests
//!
//! Tests each trait derived by `LightAccount` macro for `NonCopyRecord`:
//! - LightHasherSha -> DataHasher + ToByteArray
//! - LightDiscriminator -> LIGHT_DISCRIMINATOR constant
//! - Compressible -> HasCompressionInfo + CompressAs + Size + CompressedInitSpace
//!
//! Note: Since NonCopyRecord has no Pubkey fields, the Pack trait generates an identity
//! implementation where Packed = Self. String fields use the clone() code path in pack/unpack.
//! Therefore, no Pack/Unpack tests are needed.

use csdk_anchor_full_derived_test::NonCopyRecord;
use light_account::{CompressAs, CompressionInfo};
use light_hasher::{DataHasher, Sha256};

use super::shared::CompressibleTestFactory;
use crate::generate_trait_tests;

// =============================================================================
// Factory Implementation
// =============================================================================

impl CompressibleTestFactory for NonCopyRecord {
    fn with_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::default(),
            name: "test name".to_string(),
            description: "test description".to_string(),
            counter: 0,
        }
    }

    fn without_compression_info() -> Self {
        Self {
            compression_info: CompressionInfo::compressed(),
            name: "test name".to_string(),
            description: "test description".to_string(),
            counter: 0,
        }
    }
}

// =============================================================================
// Generate all generic trait tests via macro
// =============================================================================

generate_trait_tests!(NonCopyRecord);

// =============================================================================
// Struct-Specific CompressAs Tests
// =============================================================================

#[test]
fn test_compress_as_preserves_other_fields() {
    let name = "Alice".to_string();
    let description = "A test user".to_string();
    let counter = 999u64;

    let record = NonCopyRecord {
        compression_info: CompressionInfo::default(),
        name: name.clone(),
        description: description.clone(),
        counter,
    };

    let compressed = record.compress_as();
    assert_eq!(compressed.name, name);
    assert_eq!(compressed.description, description);
    assert_eq!(compressed.counter, counter);
}

#[test]
fn test_compress_as_when_compression_info_already_none() {
    let name = "Bob".to_string();
    let description = "Another test user".to_string();
    let counter = 123u64;

    let record = NonCopyRecord {
        compression_info: CompressionInfo::compressed(),
        name: name.clone(),
        description: description.clone(),
        counter,
    };

    let compressed = record.compress_as();

    // Should still work and preserve fields
    assert_eq!(compressed.name, name);
    assert_eq!(compressed.description, description);
    assert_eq!(compressed.counter, counter);
}

// =============================================================================
// Struct-Specific DataHasher Tests
// =============================================================================

#[test]
fn test_hash_differs_for_different_counter() {
    let record1 = NonCopyRecord {
        compression_info: CompressionInfo::compressed(),
        name: "test".to_string(),
        description: "description".to_string(),
        counter: 1,
    };

    let record2 = NonCopyRecord {
        compression_info: CompressionInfo::compressed(),
        name: "test".to_string(),
        description: "description".to_string(),
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
fn test_hash_differs_for_different_name() {
    let record1 = NonCopyRecord {
        compression_info: CompressionInfo::compressed(),
        name: "Alice".to_string(),
        description: "description".to_string(),
        counter: 100,
    };

    let record2 = NonCopyRecord {
        compression_info: CompressionInfo::compressed(),
        name: "Bob".to_string(),
        description: "description".to_string(),
        counter: 100,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(hash1, hash2, "different name should produce different hash");
}

#[test]
fn test_hash_differs_for_different_description() {
    let record1 = NonCopyRecord {
        compression_info: CompressionInfo::compressed(),
        name: "test".to_string(),
        description: "first description".to_string(),
        counter: 100,
    };

    let record2 = NonCopyRecord {
        compression_info: CompressionInfo::compressed(),
        name: "test".to_string(),
        description: "second description".to_string(),
        counter: 100,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different description should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_different_string_length() {
    let record1 = NonCopyRecord {
        compression_info: CompressionInfo::compressed(),
        name: "a".to_string(),
        description: "description".to_string(),
        counter: 100,
    };

    let record2 = NonCopyRecord {
        compression_info: CompressionInfo::compressed(),
        name: "aa".to_string(),
        description: "description".to_string(),
        counter: 100,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "different string length should produce different hash"
    );
}

#[test]
fn test_hash_differs_for_empty_vs_non_empty_string() {
    let record1 = NonCopyRecord {
        compression_info: CompressionInfo::compressed(),
        name: "".to_string(),
        description: "description".to_string(),
        counter: 100,
    };

    let record2 = NonCopyRecord {
        compression_info: CompressionInfo::compressed(),
        name: "name".to_string(),
        description: "description".to_string(),
        counter: 100,
    };

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "empty vs non-empty string should produce different hash"
    );
}
