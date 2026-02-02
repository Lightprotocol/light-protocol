//! Regression tests for address derivation functions.
//!
//! These tests ensure that address derivation produces stable, expected results
//! across SDK versions. Any change to these values indicates a breaking change
//! in address derivation.

use light_sdk::address::{v1, AddressSeed};
use solana_pubkey::Pubkey;

// ============================================================================
// V1 Address Derivation Tests
// ============================================================================

/// Regression test for v1::derive_address_seed with single seed.
#[test]
fn test_v1_derive_address_seed_single() {
    let program_id = Pubkey::new_from_array([
        100, 107, 175, 177, 40, 13, 216, 39, 157, 127, 44, 88, 81, 65, 139, 243, 208, 214, 99, 121,
        7, 157, 114, 42, 73, 26, 197, 102, 50, 36, 40, 122,
    ]); // "7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz"

    let address_seed = v1::derive_address_seed(&[b"counter"], &program_id);

    let expected_seed: [u8; 32] = [
        0, 245, 19, 201, 93, 115, 34, 4, 40, 137, 210, 14, 49, 244, 116, 217, 75, 141, 75, 174, 91,
        204, 52, 232, 23, 205, 206, 11, 156, 153, 138, 2,
    ];

    assert_eq!(
        address_seed,
        AddressSeed::from(expected_seed),
        "v1::derive_address_seed should produce expected hash for single seed"
    );
}

/// Regression test for v1::derive_address_seed with multiple seeds.
#[test]
fn test_v1_derive_address_seed_multiple() {
    let program_id = Pubkey::new_from_array([
        100, 107, 175, 177, 40, 13, 216, 39, 157, 127, 44, 88, 81, 65, 139, 243, 208, 214, 99, 121,
        7, 157, 114, 42, 73, 26, 197, 102, 50, 36, 40, 122,
    ]); // "7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz"

    let address_seed = v1::derive_address_seed(&[b"foo", b"bar"], &program_id);

    let expected_seed: [u8; 32] = [
        0, 144, 35, 68, 111, 204, 23, 151, 120, 31, 223, 158, 197, 136, 5, 247, 175, 29, 75, 0, 98,
        141, 6, 70, 59, 251, 227, 126, 157, 101, 113, 15,
    ];

    assert_eq!(
        address_seed,
        AddressSeed::from(expected_seed),
        "v1::derive_address_seed should produce expected hash for multiple seeds"
    );
}

/// Regression test for v1::derive_address (full address derivation).
#[test]
fn test_v1_derive_address() {
    let program_id = Pubkey::new_from_array([
        100, 107, 175, 177, 40, 13, 216, 39, 157, 127, 44, 88, 81, 65, 139, 243, 208, 214, 99, 121,
        7, 157, 114, 42, 73, 26, 197, 102, 50, 36, 40, 122,
    ]); // "7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz"

    let address_tree_pubkey = Pubkey::new_from_array([0u8; 32]);

    let (address, address_seed) =
        v1::derive_address(&[b"foo", b"bar"], &address_tree_pubkey, &program_id);

    let expected_seed: [u8; 32] = [
        0, 144, 35, 68, 111, 204, 23, 151, 120, 31, 223, 158, 197, 136, 5, 247, 175, 29, 75, 0, 98,
        141, 6, 70, 59, 251, 227, 126, 157, 101, 113, 15,
    ];

    let expected_address: [u8; 32] = [
        0, 76, 248, 62, 238, 197, 1, 141, 147, 231, 141, 73, 114, 55, 148, 180, 248, 40, 93, 185,
        22, 21, 249, 166, 123, 52, 176, 211, 176, 181, 40, 137,
    ];

    assert_eq!(
        address_seed,
        AddressSeed::from(expected_seed),
        "v1::derive_address should produce expected seed"
    );
    assert_eq!(
        address, expected_address,
        "v1::derive_address should produce expected address"
    );
}

/// Regression test for v1::derive_address with non-zero address tree.
#[test]
fn test_v1_derive_address_nonzero_tree() {
    let program_id = Pubkey::new_from_array([
        100, 107, 175, 177, 40, 13, 216, 39, 157, 127, 44, 88, 81, 65, 139, 243, 208, 214, 99, 121,
        7, 157, 114, 42, 73, 26, 197, 102, 50, 36, 40, 122,
    ]); // "7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz"

    // Non-zero address tree pubkey
    let address_tree_pubkey = Pubkey::new_from_array([1u8; 32]);

    let (address, address_seed) =
        v1::derive_address(&[b"foo", b"bar"], &address_tree_pubkey, &program_id);

    let expected_seed: [u8; 32] = [
        0, 144, 35, 68, 111, 204, 23, 151, 120, 31, 223, 158, 197, 136, 5, 247, 175, 29, 75, 0, 98,
        141, 6, 70, 59, 251, 227, 126, 157, 101, 113, 15,
    ];

    let expected_address: [u8; 32] = [
        0, 255, 198, 80, 93, 192, 235, 41, 155, 22, 132, 77, 249, 213, 151, 62, 5, 48, 131, 228,
        84, 7, 246, 208, 228, 186, 166, 253, 226, 207, 140, 63,
    ];

    assert_eq!(
        address_seed,
        AddressSeed::from(expected_seed),
        "Seed should be independent of address tree"
    );
    assert_eq!(
        address, expected_address,
        "Address should change with different tree"
    );
}

// ============================================================================
// V2 Address Derivation Tests (requires v2 feature)
// ============================================================================

#[cfg(feature = "v2")]
mod v2_tests {
    use light_sdk::address::{v2, AddressSeed};
    use solana_pubkey::Pubkey;

    /// Regression test for v2::derive_address_seed with single seed.
    #[test]
    fn test_v2_derive_address_seed_single() {
        let address_seed = v2::derive_address_seed(&[b"counter"]);

        let expected_seed: [u8; 32] = [
            0, 165, 27, 203, 187, 69, 194, 192, 180, 210, 48, 0, 52, 246, 251, 212, 224, 61, 66,
            41, 49, 191, 123, 103, 166, 56, 32, 4, 195, 249, 84, 184,
        ];

        assert_eq!(
            address_seed,
            AddressSeed::from(expected_seed),
            "v2::derive_address_seed should produce expected hash for single seed"
        );
    }

    /// Regression test for v2::derive_address_seed with multiple seeds.
    #[test]
    fn test_v2_derive_address_seed_multiple() {
        let address_seed = v2::derive_address_seed(&[b"foo", b"bar"]);

        let expected_seed: [u8; 32] = [
            0, 177, 134, 198, 24, 76, 116, 207, 56, 127, 189, 181, 87, 237, 154, 181, 246, 54, 131,
            21, 150, 248, 106, 75, 26, 80, 147, 245, 3, 23, 136, 56,
        ];

        assert_eq!(
            address_seed,
            AddressSeed::from(expected_seed),
            "v2::derive_address_seed should produce expected hash for multiple seeds"
        );
    }

    /// Regression test for v2::derive_address_from_seed.
    #[test]
    fn test_v2_derive_address_from_seed() {
        let program_id = Pubkey::new_from_array([
            100, 107, 175, 177, 40, 13, 216, 39, 157, 127, 44, 88, 81, 65, 139, 243, 208, 214, 99,
            121, 7, 157, 114, 42, 73, 26, 197, 102, 50, 36, 40, 122,
        ]); // "7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz"

        let address_tree_pubkey = Pubkey::new_from_array([0u8; 32]);

        // Pre-computed seed for ["foo", "bar"]
        let address_seed = AddressSeed::from([
            0, 177, 134, 198, 24, 76, 116, 207, 56, 127, 189, 181, 87, 237, 154, 181, 246, 54, 131,
            21, 150, 248, 106, 75, 26, 80, 147, 245, 3, 23, 136, 56,
        ]);

        let address =
            v2::derive_address_from_seed(&address_seed, &address_tree_pubkey, &program_id);

        let expected_address: [u8; 32] = [
            0, 132, 78, 228, 232, 12, 252, 191, 251, 208, 23, 174, 212, 63, 254, 118, 101, 12, 78,
            228, 149, 165, 165, 63, 78, 36, 207, 250, 77, 97, 137, 145,
        ];

        assert_eq!(
            address, expected_address,
            "v2::derive_address_from_seed should produce expected address"
        );
    }

    /// Regression test for v2::derive_address (full address derivation).
    #[test]
    fn test_v2_derive_address() {
        let program_id = Pubkey::new_from_array([
            100, 107, 175, 177, 40, 13, 216, 39, 157, 127, 44, 88, 81, 65, 139, 243, 208, 214, 99,
            121, 7, 157, 114, 42, 73, 26, 197, 102, 50, 36, 40, 122,
        ]); // "7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz"

        let address_tree_pubkey = Pubkey::new_from_array([0u8; 32]);

        let (address, address_seed) =
            v2::derive_address(&[b"foo", b"bar"], &address_tree_pubkey, &program_id);

        let expected_seed: [u8; 32] = [
            0, 177, 134, 198, 24, 76, 116, 207, 56, 127, 189, 181, 87, 237, 154, 181, 246, 54, 131,
            21, 150, 248, 106, 75, 26, 80, 147, 245, 3, 23, 136, 56,
        ];

        let expected_address: [u8; 32] = [
            0, 132, 78, 228, 232, 12, 252, 191, 251, 208, 23, 174, 212, 63, 254, 118, 101, 12, 78,
            228, 149, 165, 165, 63, 78, 36, 207, 250, 77, 97, 137, 145,
        ];

        assert_eq!(
            address_seed,
            AddressSeed::from(expected_seed),
            "v2::derive_address should produce expected seed"
        );
        assert_eq!(
            address, expected_address,
            "v2::derive_address should produce expected address"
        );
    }

    /// Regression test for v2::derive_compressed_address (PDA-based derivation).
    #[test]
    fn test_v2_derive_compressed_address() {
        let program_id = Pubkey::new_from_array([
            100, 107, 175, 177, 40, 13, 216, 39, 157, 127, 44, 88, 81, 65, 139, 243, 208, 214, 99,
            121, 7, 157, 114, 42, 73, 26, 197, 102, 50, 36, 40, 122,
        ]); // "7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz"

        let address_tree_pubkey = Pubkey::new_from_array([0u8; 32]);

        // Use a PDA-like account address
        let account_address = Pubkey::new_from_array([42u8; 32]);

        let address =
            v2::derive_compressed_address(&account_address, &address_tree_pubkey, &program_id);

        let expected_address: [u8; 32] = [
            0, 105, 30, 171, 212, 105, 4, 106, 75, 153, 240, 54, 131, 59, 249, 62, 190, 30, 127,
            237, 32, 34, 95, 178, 183, 217, 64, 102, 144, 199, 78, 77,
        ];

        assert_eq!(
            address, expected_address,
            "v2::derive_compressed_address should produce expected address"
        );
    }

    /// Regression test for v2::derive_address with different tree.
    #[test]
    fn test_v2_derive_address_different_tree() {
        let program_id = Pubkey::new_from_array([
            100, 107, 175, 177, 40, 13, 216, 39, 157, 127, 44, 88, 81, 65, 139, 243, 208, 214, 99,
            121, 7, 157, 114, 42, 73, 26, 197, 102, 50, 36, 40, 122,
        ]); // "7yucc7fL3JGbyMwg4neUaenNSdySS39hbAk89Ao3t1Hz"

        // Non-zero address tree
        let address_tree_pubkey = Pubkey::new_from_array([1u8; 32]);

        let (address, address_seed) =
            v2::derive_address(&[b"foo", b"bar"], &address_tree_pubkey, &program_id);

        let expected_seed: [u8; 32] = [
            0, 177, 134, 198, 24, 76, 116, 207, 56, 127, 189, 181, 87, 237, 154, 181, 246, 54, 131,
            21, 150, 248, 106, 75, 26, 80, 147, 245, 3, 23, 136, 56,
        ];

        let expected_address: [u8; 32] = [
            0, 206, 50, 238, 53, 179, 169, 71, 26, 123, 239, 155, 15, 63, 61, 61, 211, 48, 90, 217,
            119, 136, 77, 242, 208, 202, 252, 217, 54, 19, 114, 55,
        ];

        assert_eq!(
            address_seed,
            AddressSeed::from(expected_seed),
            "v2 seed should be independent of address tree"
        );
        assert_eq!(
            address, expected_address,
            "v2 address should change with different tree"
        );
    }

    /// Verify v1 and v2 produce DIFFERENT results for same inputs.
    /// This documents the intentional difference between versions.
    #[test]
    fn test_v1_v2_differ() {
        use light_sdk::address::v1;

        let program_id = Pubkey::new_from_array([
            100, 107, 175, 177, 40, 13, 216, 39, 157, 127, 44, 88, 81, 65, 139, 243, 208, 214, 99,
            121, 7, 157, 114, 42, 73, 26, 197, 102, 50, 36, 40, 122,
        ]);

        let seeds: &[&[u8]] = &[b"foo", b"bar"];

        let v1_seed = v1::derive_address_seed(seeds, &program_id);
        let v2_seed = v2::derive_address_seed(seeds);

        // V1 and V2 use different hashing schemes
        assert_ne!(
            v1_seed, v2_seed,
            "v1 and v2 should produce different seeds (v1 includes program_id, v2 does not)"
        );
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test that byte 0 is always 0 (BN254 field size constraint).
#[test]
fn test_address_seed_first_byte_zero() {
    let program_id = Pubkey::new_from_array([255u8; 32]);

    // Try various seeds to ensure first byte is always 0
    for i in 0..10 {
        let seed = format!("test_seed_{}", i);
        let address_seed = v1::derive_address_seed(&[seed.as_bytes()], &program_id);
        assert_eq!(
            address_seed.0[0], 0,
            "First byte must be 0 for BN254 compatibility"
        );
    }
}

/// Test that address first byte is within BN254 field (can be non-zero but < 48).
#[test]
fn test_address_first_byte_bn254() {
    let program_id = Pubkey::new_from_array([1u8; 32]);
    let address_tree_pubkey = Pubkey::new_from_array([2u8; 32]);

    // The address derivation uses a different truncation that allows first byte < 48
    for i in 0..10 {
        let seed = format!("test_seed_{}", i);
        let (address, _) =
            v1::derive_address(&[seed.as_bytes()], &address_tree_pubkey, &program_id);
        // BN254 field modulus starts with ~48, so first byte should be < 48
        assert!(
            address[0] < 48,
            "First byte must be < 48 for BN254 compatibility, got {}",
            address[0]
        );
    }
}

/// Test empty seeds behavior.
#[test]
fn test_empty_seeds() {
    let program_id = Pubkey::new_from_array([1u8; 32]);

    let address_seed = v1::derive_address_seed(&[], &program_id);

    // Empty seeds should still produce a valid hash
    assert_eq!(
        address_seed.0[0], 0,
        "First byte must be 0 even with empty seeds"
    );

    // Should be deterministic
    let address_seed2 = v1::derive_address_seed(&[], &program_id);
    assert_eq!(
        address_seed, address_seed2,
        "Same inputs should produce same output"
    );
}
