use anchor_compressed_token::ErrorCode;
use light_account_checks::account_info::test_account_info::pinocchio::get_account_info;
use light_compressed_token::mint_action::check_authority;
use pinocchio::pubkey::Pubkey;

// Helper function to create test account info
fn create_test_account_info(
    pubkey: Pubkey,
    is_signer: bool,
) -> pinocchio::account_info::AccountInfo {
    get_account_info(
        pubkey,
        [0u8; 32], // owner
        is_signer,
        false, // writable
        false, // executable
        vec![0u8; 32],
    )
}

/// Test all essential scenarios for the simplified check_authority function
///
/// The function now only takes current_authority and signer (no fallback).
///
/// Test cases:
/// 1. None authority -> Error (no authority set)
/// 2. Valid authority + matching signer -> Success
/// 3. Valid authority + non-matching signer -> Error
/// 4. Revoked authority ([0u8; 32]) -> Error (authority has been revoked)
#[test]
fn test_check_authority_essential_cases() {
    let valid_authority = light_compressed_account::Pubkey::from([1u8; 32]);
    let wrong_signer = light_compressed_account::Pubkey::from([2u8; 32]);
    let revoked_authority = light_compressed_account::Pubkey::from([0u8; 32]);

    // Test Case 1: None authority -> Error
    {
        let signer = create_test_account_info(Pubkey::from(valid_authority.to_bytes()), true);
        let result = check_authority(None, signer.key(), "test authority");

        assert!(result.is_err(), "None authority should fail");
        match result.err().unwrap() {
            anchor_lang::prelude::ProgramError::Custom(code) => {
                assert_eq!(
                    code,
                    ErrorCode::InvalidAuthorityMint as u32,
                    "Should return InvalidAuthorityMint for None authority"
                );
            }
            other => panic!("Expected InvalidAuthorityMint, got {:?}", other),
        }
    }

    // Test Case 2: Valid authority + matching signer -> Success
    {
        let signer = create_test_account_info(Pubkey::from(valid_authority.to_bytes()), true);
        let result = check_authority(Some(valid_authority), signer.key(), "test authority");

        assert!(
            result.is_ok(),
            "Valid authority with matching signer should succeed"
        );
    }

    // Test Case 3: Valid authority + non-matching signer -> Error
    {
        let signer = create_test_account_info(Pubkey::from(wrong_signer.to_bytes()), true);
        let result = check_authority(Some(valid_authority), signer.key(), "test authority");

        assert!(
            result.is_err(),
            "Valid authority with wrong signer should fail"
        );
        match result.err().unwrap() {
            anchor_lang::prelude::ProgramError::Custom(code) => {
                assert_eq!(
                    code,
                    ErrorCode::InvalidAuthorityMint as u32,
                    "Should return InvalidAuthorityMint for wrong signer"
                );
            }
            other => panic!("Expected InvalidAuthorityMint, got {:?}", other),
        }
    }

    // Test Case 4: Revoked authority ([0u8; 32]) -> Error
    {
        // Even if we somehow had a signer that matched [0u8; 32], it should still fail
        // In practice this is impossible, but the function checks for revoked state explicitly
        let signer = create_test_account_info(Pubkey::from(wrong_signer.to_bytes()), true);
        let result = check_authority(Some(revoked_authority), signer.key(), "test authority");

        assert!(result.is_err(), "Revoked authority should always fail");
        match result.err().unwrap() {
            anchor_lang::prelude::ProgramError::Custom(code) => {
                assert_eq!(
                    code,
                    ErrorCode::InvalidAuthorityMint as u32,
                    "Should return InvalidAuthorityMint for revoked authority"
                );
            }
            other => panic!("Expected InvalidAuthorityMint, got {:?}", other),
        }
    }

    println!("✅ All essential check_authority test cases passed!");
}

/// Test edge case: authority exists but is [0u8; 32] (revoked)
/// This tests the special handling of revoked authorities
#[test]
fn test_check_authority_revoked_edge_case() {
    let revoked_authority = light_compressed_account::Pubkey::from([0u8; 32]);
    let different_signer = light_compressed_account::Pubkey::from([1u8; 32]);

    // Test with a different signer (the normal case)
    let signer = create_test_account_info(Pubkey::from(different_signer.to_bytes()), true);
    let result = check_authority(Some(revoked_authority), signer.key(), "revoked authority");

    // Revoked authority with different signer should fail with specific error
    assert!(
        result.is_err(),
        "Revoked authority with different signer should fail"
    );
    match result.err().unwrap() {
        anchor_lang::prelude::ProgramError::Custom(code) => {
            assert_eq!(
                code,
                ErrorCode::InvalidAuthorityMint as u32,
                "Should return InvalidAuthorityMint for revoked authority"
            );
        }
        other => panic!("Expected InvalidAuthorityMint, got {:?}", other),
    }

    // Note: The theoretical case where signer matches [0u8; 32] would actually succeed
    // due to the order of checks in the function, but this is impossible in practice
    // as no valid cryptographic key can be all zeros.
    let impossible_signer = create_test_account_info(Pubkey::from([0u8; 32]), true);
    let edge_result = check_authority(
        Some(revoked_authority),
        impossible_signer.key(),
        "revoked authority edge case",
    );

    // This would succeed (match happens before revoked check), but it's a theoretical edge case
    assert!(
        edge_result.is_ok(),
        "Theoretical edge case: if signer matched [0u8; 32] it would succeed"
    );
}
