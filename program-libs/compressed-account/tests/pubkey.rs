#![cfg(all(test, feature = "new-unique"))]

use light_compressed_account::pubkey::Pubkey;
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use rand::{rngs::StdRng, thread_rng, Rng, SeedableRng};

/// Tests for all Pubkey zero-copy functions:
/// 1. new_zero_copy() - Creates zero-copy mutable reference via ZeroCopyNew trait (wraps zero_copy_at_mut)
/// 2. zero_copy_at_mut() - Creates zero-copy mutable reference via ZeroCopyAtMut trait
/// 3. zero_copy_at() - Creates zero-copy read-only reference via ZeroCopyAt trait
/// - Success cases: valid byte arrays (>=32 bytes)
/// - Error cases: insufficient bytes (<32 bytes), including zero-byte edge case
/// - Mutability: modifications through mutable zero-copy references affect original bytes
/// - Read-only: zero_copy_at provides immutable access to byte data

#[test]
fn test_pubkey_zero_copy_functions() {
    // Generate seed with thread_rng() and print for reproducibility
    let seed = thread_rng().gen::<u64>();
    println!("Pubkey zero-copy test seed: {}", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    // Execute: 1000 iterations testing both functions
    for _ in 0..1000 {
        // Generate random test parameters
        let extra_bytes_len = rng.gen_range(0..=64);
        let total_len = 32 + extra_bytes_len;

        // Create random byte array with pubkey (32 bytes) + extra bytes
        let test_bytes: Vec<u8> = (0..total_len).map(|_| rng.gen::<u8>()).collect();
        let expected_pubkey_bytes: [u8; 32] = test_bytes[0..32].try_into().unwrap();
        let expected_remaining_len = extra_bytes_len;

        // Test 1: All three zero-copy functions with same input
        {
            let mut bytes_copy1 = test_bytes.clone();
            let mut bytes_copy2 = test_bytes.clone();
            let bytes_copy3 = test_bytes.clone();

            // Test new_zero_copy() - wraps zero_copy_at_mut()
            let result1 = Pubkey::new_zero_copy(&mut bytes_copy1, ());
            assert!(
                result1.is_ok(),
                "new_zero_copy should succeed with {} bytes",
                total_len
            );
            let (z_pubkey1, remaining1) = result1.unwrap();

            // Test zero_copy_at_mut() - mutable reference
            let result2 = Pubkey::zero_copy_at_mut(&mut bytes_copy2);
            assert!(
                result2.is_ok(),
                "zero_copy_at_mut should succeed with {} bytes",
                total_len
            );
            let (z_pubkey2, remaining2) = result2.unwrap();

            // Test zero_copy_at() - read-only reference
            let result3 = Pubkey::zero_copy_at(&bytes_copy3);
            assert!(
                result3.is_ok(),
                "zero_copy_at should succeed with {} bytes",
                total_len
            );
            let (z_pubkey3, remaining3) = result3.unwrap();

            // All functions should return identical pubkey bytes
            assert_eq!(z_pubkey1.to_bytes(), expected_pubkey_bytes);
            assert_eq!(z_pubkey2.to_bytes(), expected_pubkey_bytes);
            assert_eq!(z_pubkey3.to_bytes(), expected_pubkey_bytes);

            // All functions should return same remaining bytes length
            assert_eq!(remaining1.len(), expected_remaining_len);
            assert_eq!(remaining2.len(), expected_remaining_len);
            assert_eq!(remaining3.len(), expected_remaining_len);

            // All functions should return same remaining bytes content
            if extra_bytes_len > 0 {
                assert_eq!(remaining1, &test_bytes[32..]);
                assert_eq!(remaining2, &test_bytes[32..]);
                assert_eq!(remaining3, &test_bytes[32..]);
            }
        }

        // Test 2: Mutability verification - modifications through zero-copy affect original
        {
            let mut bytes_copy = test_bytes.clone();
            let new_pubkey_bytes: [u8; 32] = rng.gen();

            {
                let (mut z_pubkey, _) = Pubkey::zero_copy_at_mut(&mut bytes_copy).unwrap();

                // Modify through zero-copy reference
                *z_pubkey = Pubkey::new_from_array(new_pubkey_bytes);
                assert_eq!(z_pubkey.to_bytes(), new_pubkey_bytes);
            } // Drop z_pubkey reference here

            // Verify original bytes were modified
            assert_eq!(bytes_copy[0..32], new_pubkey_bytes);
        }

        // Test 3: Error cases - insufficient bytes
        if rng.gen_bool(0.1) {
            // 10% chance to test error cases
            let insufficient_len = rng.gen_range(0..32);
            let mut insufficient_bytes1: Vec<u8> =
                (0..insufficient_len).map(|_| rng.gen::<u8>()).collect();
            let mut insufficient_bytes2 = insufficient_bytes1.clone();
            let insufficient_bytes3 = insufficient_bytes1.clone();

            // All functions should fail with insufficient bytes and return proper errors
            let result1 = Pubkey::new_zero_copy(&mut insufficient_bytes1, ());
            let result2 = Pubkey::zero_copy_at_mut(&mut insufficient_bytes2);
            let result3 = Pubkey::zero_copy_at(&insufficient_bytes3);

            assert!(
                result1.is_err(),
                "new_zero_copy should fail with {} bytes",
                insufficient_len
            );
            assert!(
                result2.is_err(),
                "zero_copy_at_mut should fail with {} bytes",
                insufficient_len
            );
            assert!(
                result3.is_err(),
                "zero_copy_at should fail with {} bytes",
                insufficient_len
            );

            // All should return the same error type (zerocopy conversion error)
            let error1 = result1.unwrap_err();
            let error2 = result2.unwrap_err();
            let error3 = result3.unwrap_err();
            assert_eq!(
                std::mem::discriminant(&error1),
                std::mem::discriminant(&error2)
            );
            assert_eq!(
                std::mem::discriminant(&error2),
                std::mem::discriminant(&error3)
            );
        }
    }

    // Test 4: Edge cases - exact 32 bytes and zero bytes
    {
        // Test exact 32 bytes
        let exact_bytes: Vec<u8> = (0..32).map(|i| i as u8).collect();
        let expected_bytes: [u8; 32] = exact_bytes.clone().try_into().unwrap();

        // All functions should succeed with exactly 32 bytes
        let mut exact_bytes1 = exact_bytes.clone();
        let mut exact_bytes2 = exact_bytes.clone();
        let exact_bytes3 = exact_bytes.clone();

        let (z_pubkey1, remaining1) = Pubkey::new_zero_copy(&mut exact_bytes1, ()).unwrap();
        let (z_pubkey2, remaining2) = Pubkey::zero_copy_at_mut(&mut exact_bytes2).unwrap();
        let (z_pubkey3, remaining3) = Pubkey::zero_copy_at(&exact_bytes3).unwrap();

        assert_eq!(z_pubkey1.to_bytes(), expected_bytes);
        assert_eq!(z_pubkey2.to_bytes(), expected_bytes);
        assert_eq!(z_pubkey3.to_bytes(), expected_bytes);
        assert_eq!(remaining1.len(), 0);
        assert_eq!(remaining2.len(), 0);
        assert_eq!(remaining3.len(), 0);
    }

    // Test 5: Zero bytes edge case - critical boundary condition
    {
        let mut empty_bytes1: Vec<u8> = vec![];
        let mut empty_bytes2: Vec<u8> = vec![];
        let empty_bytes3: Vec<u8> = vec![];

        // All functions should fail with zero bytes
        let result1 = Pubkey::new_zero_copy(&mut empty_bytes1, ());
        let result2 = Pubkey::zero_copy_at_mut(&mut empty_bytes2);
        let result3 = Pubkey::zero_copy_at(&empty_bytes3);

        // Assert specific error types, not just failure - should be Size error for insufficient bytes
        match result1.unwrap_err() {
            light_zero_copy::errors::ZeroCopyError::Size => (), // Expected size error for insufficient bytes
            other => panic!("Expected Size error for zero bytes, got: {:?}", other),
        }
        match result2.unwrap_err() {
            light_zero_copy::errors::ZeroCopyError::Size => (), // Expected size error for insufficient bytes
            other => panic!("Expected Size error for zero bytes, got: {:?}", other),
        }
        match result3.unwrap_err() {
            light_zero_copy::errors::ZeroCopyError::Size => (), // Expected size error for insufficient bytes
            other => panic!("Expected Size error for zero bytes, got: {:?}", other),
        }
    }
}
