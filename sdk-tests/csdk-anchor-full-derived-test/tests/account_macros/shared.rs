//! Shared generic test helpers for LightAccount-derived traits.
//!
//! These functions test trait implementations generically and can be reused
//! across different account struct types.

use std::borrow::Cow;

use light_account::Size;
use light_hasher::{DataHasher, Sha256};
use light_account::{CompressAs, CompressedInitSpace, CompressionState, HasCompressionInfo};
use light_sdk::LightDiscriminator;

// =============================================================================
// Test Factory Trait
// =============================================================================

/// Trait for creating test instances of compressible account structs.
///
/// Implement this trait for each account struct to enable generic testing.
pub trait CompressibleTestFactory: Sized {
    /// Create an instance with `compression_info = CompressionInfo::default()`
    fn with_compression_info() -> Self;

    /// Create an instance with `compression_info = CompressionInfo::compressed()`
    fn without_compression_info() -> Self;
}

// =============================================================================
// LightDiscriminator Tests (4 tests)
// =============================================================================

/// Verifies LIGHT_DISCRIMINATOR is exactly 8 bytes.
pub fn assert_discriminator_is_8_bytes<T: LightDiscriminator>() {
    let discriminator = T::LIGHT_DISCRIMINATOR;
    assert_eq!(
        discriminator.len(),
        8,
        "LIGHT_DISCRIMINATOR should be 8 bytes"
    );
}

/// Verifies LIGHT_DISCRIMINATOR is not all zeros.
pub fn assert_discriminator_is_non_zero<T: LightDiscriminator>() {
    let discriminator = T::LIGHT_DISCRIMINATOR;
    let all_zero = discriminator.iter().all(|&b| b == 0);
    assert!(!all_zero, "LIGHT_DISCRIMINATOR should not be all zeros");
}

/// Verifies discriminator() method returns the same value as LIGHT_DISCRIMINATOR constant.
pub fn assert_discriminator_method_matches_constant<T: LightDiscriminator>() {
    let from_method = T::discriminator();
    let from_constant = T::LIGHT_DISCRIMINATOR;
    assert_eq!(
        from_method, from_constant,
        "discriminator() should return LIGHT_DISCRIMINATOR"
    );
}

/// Verifies LIGHT_DISCRIMINATOR_SLICE matches the LIGHT_DISCRIMINATOR array.
pub fn assert_discriminator_slice_matches_array<T: LightDiscriminator>() {
    let array = T::LIGHT_DISCRIMINATOR;
    let slice = T::LIGHT_DISCRIMINATOR_SLICE;

    assert_eq!(
        slice, &array,
        "LIGHT_DISCRIMINATOR_SLICE should match LIGHT_DISCRIMINATOR array"
    );
    assert_eq!(slice.len(), 8);
}

// =============================================================================
// HasCompressionInfo Tests (6 tests)
// =============================================================================

/// Verifies compression_info() returns a valid reference.
pub fn assert_compression_info_returns_reference<
    T: HasCompressionInfo + CompressibleTestFactory,
>() {
    let record = T::with_compression_info();
    let info = record
        .compression_info()
        .expect("compression_info should return Ok");
    // Just verify we can access it - the default values
    assert_eq!(info.config_version, 0);
    assert_eq!(info.lamports_per_write, 0);
}

/// Verifies compression_info_mut() allows modification.
pub fn assert_compression_info_mut_allows_modification<
    T: HasCompressionInfo + CompressibleTestFactory,
>() {
    let mut record = T::with_compression_info();

    {
        let info = record
            .compression_info_mut()
            .expect("compression_info should return Ok");
        info.config_version = 99;
        info.lamports_per_write = 1000;
    }

    assert_eq!(
        record
            .compression_info()
            .expect("compression_info should return Ok")
            .config_version,
        99
    );
    assert_eq!(
        record
            .compression_info()
            .expect("compression_info should return Ok")
            .lamports_per_write,
        1000
    );
}

/// Verifies set_compression_info_none() sets the field to CompressionInfo::compressed().
pub fn assert_set_compression_info_none_works<T: HasCompressionInfo + CompressibleTestFactory>() {
    let mut record = T::with_compression_info();

    // Verify it starts with default compression_info
    let initial = record
        .compression_info()
        .expect("compression_info should return Ok");
    assert_eq!(initial.state, light_account::CompressionState::default());

    record
        .set_compression_info_none()
        .expect("set_compression_info_none should succeed");

    // Verify it's now compressed
    let final_info = record
        .compression_info()
        .expect("compression_info should return Ok");
    assert_eq!(final_info.state, CompressionState::Compressed);
}

/// Verifies compression_info() returns Ok with Compressed state when set to none.
pub fn assert_compression_info_returns_ok_when_none<
    T: HasCompressionInfo + CompressibleTestFactory,
>() {
    let record = T::without_compression_info();
    // compression_info() should return Ok
    let info = record
        .compression_info()
        .expect("compression_info should return Ok");
    // Verify it's in Compressed state
    assert_eq!(info.state, CompressionState::Compressed);
}

/// Verifies compression_info_mut() returns Ok with Compressed state when set to none.
pub fn assert_compression_info_mut_returns_ok_when_none<
    T: HasCompressionInfo + CompressibleTestFactory,
>() {
    let mut record = T::without_compression_info();
    // compression_info_mut() should return Ok
    let info = record
        .compression_info_mut()
        .expect("compression_info_mut should return Ok");
    // Verify it's in Compressed state
    assert_eq!(info.state, CompressionState::Compressed);
}

// =============================================================================
// CompressAs Tests (2 tests)
// =============================================================================

/// Verifies compress_as() sets compression_info to Compressed state.
pub fn assert_compress_as_sets_compression_info_to_compressed<
    T: CompressAs<Output = T> + HasCompressionInfo + CompressibleTestFactory + Clone,
>() {
    let record = T::with_compression_info();
    let compressed = record.compress_as();

    // Get the inner value
    let inner = compressed.into_owned();
    let info = inner
        .compression_info()
        .expect("compression_info should return Ok");
    assert_eq!(
        info.state,
        CompressionState::Compressed,
        "compress_as should set compression_info to Compressed state"
    );
}

/// Verifies compress_as() returns Cow::Owned (cloned with compression_info set to Compressed).
pub fn assert_compress_as_returns_owned_cow<
    T: CompressAs<Output = T> + HasCompressionInfo + CompressibleTestFactory + Clone,
>() {
    let record = T::with_compression_info();
    let compressed = record.compress_as();

    assert!(
        matches!(compressed, Cow::Owned(_)),
        "compress_as should return Cow::Owned (cloned with compression_info set to Compressed)"
    );
}

// =============================================================================
// Size Tests (2 tests)
// =============================================================================

/// Verifies size() returns a positive value.
pub fn assert_size_returns_positive<T: Size + CompressibleTestFactory>() {
    let record = T::with_compression_info();
    let size = record.size().expect("size should succeed");
    assert!(size > 0, "size should be positive");
}

/// Verifies size() returns the same value when called multiple times on the same instance.
pub fn assert_size_is_deterministic<T: Size + CompressibleTestFactory + Clone>() {
    let record = T::with_compression_info();
    let record_clone = record.clone();

    let size1 = record.size().expect("size should succeed");
    let size2 = record_clone.size().expect("size should succeed");

    assert_eq!(size1, size2, "size should be deterministic for same data");
}

// =============================================================================
// CompressedInitSpace Tests (1 test)
// =============================================================================

/// Verifies COMPRESSED_INIT_SPACE is at least as large as the discriminator.
pub fn assert_compressed_init_space_includes_discriminator<
    T: CompressedInitSpace + LightDiscriminator,
>() {
    let compressed_space = T::COMPRESSED_INIT_SPACE;
    let discriminator_len = T::LIGHT_DISCRIMINATOR.len();

    assert!(
        compressed_space >= discriminator_len,
        "COMPRESSED_INIT_SPACE ({}) should be >= discriminator length ({})",
        compressed_space,
        discriminator_len
    );
}

// =============================================================================
// DataHasher Tests (3 tests)
// =============================================================================

/// Verifies hash() produces a 32-byte result.
pub fn assert_hash_produces_32_bytes<T: DataHasher + CompressibleTestFactory>() {
    let record = T::without_compression_info();
    let hash = record.hash::<Sha256>().expect("hash should succeed");
    assert_eq!(hash.len(), 32, "hash should produce 32-byte result");
}

/// Verifies hash() is deterministic (same input = same hash).
pub fn assert_hash_is_deterministic<T: DataHasher + CompressibleTestFactory + Clone>() {
    let record1 = T::without_compression_info();
    let record2 = record1.clone();

    let hash1 = record1.hash::<Sha256>().expect("hash should succeed");
    let hash2 = record2.hash::<Sha256>().expect("hash should succeed");

    assert_eq!(hash1, hash2, "same input should produce same hash");
}

/// Verifies compression_info IS included in the hash (LightHasherSha behavior).
pub fn assert_hash_includes_compression_info<T: DataHasher + CompressibleTestFactory>() {
    let record_with_info = T::with_compression_info();
    let record_without_info = T::without_compression_info();

    let hash1 = record_with_info
        .hash::<Sha256>()
        .expect("hash should succeed");
    let hash2 = record_without_info
        .hash::<Sha256>()
        .expect("hash should succeed");

    assert_ne!(
        hash1, hash2,
        "compression_info SHOULD affect hash - LightHasherSha hashes entire struct"
    );
}

// =============================================================================
// Macro for generating all trait tests
// =============================================================================

/// Generates all generic trait tests for a given type.
///
/// Usage:
/// ```ignore
/// generate_trait_tests!(SinglePubkeyRecord);
/// ```
#[macro_export]
macro_rules! generate_trait_tests {
    ($type:ty) => {
        mod discriminator_tests {
            use $crate::shared::*;

            use super::*;

            #[test]
            fn test_discriminator_is_8_bytes() {
                assert_discriminator_is_8_bytes::<$type>();
            }

            #[test]
            fn test_discriminator_is_non_zero() {
                assert_discriminator_is_non_zero::<$type>();
            }

            #[test]
            fn test_discriminator_method_matches_constant() {
                assert_discriminator_method_matches_constant::<$type>();
            }

            #[test]
            fn test_discriminator_slice_matches_array() {
                assert_discriminator_slice_matches_array::<$type>();
            }
        }

        mod has_compression_info_tests {
            use $crate::shared::*;

            use super::*;

            #[test]
            fn test_compression_info_returns_reference() {
                assert_compression_info_returns_reference::<$type>();
            }

            #[test]
            fn test_compression_info_mut_allows_modification() {
                assert_compression_info_mut_allows_modification::<$type>();
            }

            #[test]
            fn test_set_compression_info_none_works() {
                assert_set_compression_info_none_works::<$type>();
            }

            #[test]
            fn test_compression_info_returns_ok_when_none() {
                assert_compression_info_returns_ok_when_none::<$type>();
            }

            #[test]
            fn test_compression_info_mut_returns_ok_when_none() {
                assert_compression_info_mut_returns_ok_when_none::<$type>();
            }
        }

        mod compress_as_tests {
            use $crate::shared::*;

            use super::*;

            #[test]
            fn test_compress_as_sets_compression_info_to_compressed() {
                assert_compress_as_sets_compression_info_to_compressed::<$type>();
            }

            #[test]
            fn test_compress_as_returns_owned_cow() {
                assert_compress_as_returns_owned_cow::<$type>();
            }
        }

        mod size_tests {
            use $crate::shared::*;

            use super::*;

            #[test]
            fn test_size_returns_positive() {
                assert_size_returns_positive::<$type>();
            }

            #[test]
            fn test_size_is_deterministic() {
                assert_size_is_deterministic::<$type>();
            }
        }

        mod compressed_init_space_tests {
            use $crate::shared::*;

            use super::*;

            #[test]
            fn test_compressed_init_space_includes_discriminator() {
                assert_compressed_init_space_includes_discriminator::<$type>();
            }
        }

        mod data_hasher_tests {
            use $crate::shared::*;

            use super::*;

            #[test]
            fn test_hash_produces_32_bytes() {
                assert_hash_produces_32_bytes::<$type>();
            }

            #[test]
            fn test_hash_is_deterministic() {
                assert_hash_is_deterministic::<$type>();
            }

            #[test]
            fn test_hash_includes_compression_info() {
                assert_hash_includes_compression_info::<$type>();
            }
        }
    };
}
