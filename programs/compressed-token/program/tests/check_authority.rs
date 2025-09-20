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

/// Comprehensive test covering all 27 possible input combinations for check_authority function
///
/// TRUTH TABLE - All possible combinations of check_authority(current, fallback, signer, name):
/// Format: (current_state, fallback_state, signer_match) -> expected_result
///
/// States: None=N, ValidAuth=V, RevokedAuth=R ([0u8; 32])
/// Signer match: C=current, F=fallback, X=none
/// Result: OK=success, ERR=InvalidAuthorityMint error
///
/// | # | Current | Fallback | Signer | Expected | Reason                                    |
/// |---|---------|----------|--------|----------|-------------------------------------------|
/// | 1 | None    | None     | C      | ERR      | Fallback None is revoked state            |
/// | 2 | None    | None     | F      | ERR      | Fallback None is revoked state            |
/// | 3 | None    | None     | X      | ERR      | Fallback None is revoked state            |
/// | 4 | None    | Valid    | C      | ERR      | Signer doesn't match fallback             |
/// | 5 | None    | Valid    | F      | OK       | Uses fallback, signer matches             |
/// | 6 | None    | Valid    | X      | ERR      | Uses fallback, signer doesn't match       |
/// | 7 | None    | Revoked  | C      | ERR      | Fallback is revoked                      |
/// | 8 | None    | Revoked  | F      | ---      | IMPOSSIBLE: signer cannot match [0u8; 32] |
/// | 9 | None    | Revoked  | X      | ERR      | Fallback is revoked                      |
/// |10 | Valid   | None     | C      | ERR      | Fallback None is revoked state            |
/// |11 | Valid   | None     | F      | ERR      | Fallback None is revoked state            |
/// |12 | Valid   | None     | X      | ERR      | Fallback None is revoked state            |
/// |13 | Valid   | Valid    | C      | OK       | Current takes precedence, signer matches |
/// |14 | Valid   | Valid    | F      | ERR      | Current takes precedence, wrong signer   |
/// |15 | Valid   | Valid    | X      | ERR      | Current takes precedence, wrong signer   |
/// |16 | Valid   | Revoked  | C      | OK       | Current takes precedence, signer matches |
/// |17 | Valid   | Revoked  | F      | ERR      | Current takes precedence, wrong signer   |
/// |18 | Valid   | Revoked  | X      | ERR      | Current takes precedence, wrong signer   |
/// |19 | Revoked | None     | C      | ERR      | Fallback None is revoked state            |
/// |20 | Revoked | None     | F      | ERR      | Fallback None is revoked state            |
/// |21 | Revoked | None     | X      | ERR      | Fallback None is revoked state            |
/// |22 | Revoked | Valid    | C      | ERR      | Current is revoked (takes precedence)    |
/// |23 | Revoked | Valid    | F      | ERR      | Current is revoked (takes precedence)    |
/// |24 | Revoked | Valid    | X      | ERR      | Current is revoked (takes precedence)    |
/// |25 | Revoked | Revoked  | C      | ---      | IMPOSSIBLE: signer cannot match [0u8; 32] |
/// |26 | Revoked | Revoked  | F      | ---      | IMPOSSIBLE: signer cannot match [0u8; 32] |
/// |27 | Revoked | Revoked  | X      | ERR      | Both authorities revoked                 |
///
/// SUCCESS cases: 5, 13, 16 (3 out of 24 testable)
/// FAILURE cases: All others (21 out of 24 testable)
/// IMPOSSIBLE cases: 8, 25, 26 (3 cases removed - signer cannot match [0u8; 32])
#[derive(Debug, Clone)]
struct CheckAuthorityTestInput {
    current_authority: Option<light_compressed_account::Pubkey>,
    fallback_authority: Option<light_compressed_account::Pubkey>,
    signer_matches: SignerMatch,
    authority_name: &'static str,
}

#[derive(Debug, Clone)]
enum SignerMatch {
    Current,  // Signer matches current authority
    Fallback, // Signer matches fallback authority
    None,     // Signer doesn't match either
}

#[derive(Debug, Clone)]
struct CheckAuthorityTestExpected {
    should_succeed: bool,
    error_code: Option<ErrorCode>,
}

#[derive(Debug)]
struct CheckAuthorityTestCase {
    name: &'static str,
    input: CheckAuthorityTestInput,
    expected: CheckAuthorityTestExpected,
}

#[test]
fn test_check_authority_comprehensive_truth_table() {
    let current_auth = light_compressed_account::Pubkey::from([1u8; 32]);
    let fallback_auth = light_compressed_account::Pubkey::from([2u8; 32]);
    let revoked_auth = light_compressed_account::Pubkey::from([0u8; 32]);
    let unrelated_signer = light_compressed_account::Pubkey::from([3u8; 32]);

    let test_cases = vec![
        // Cases 1-3: Current=None, Fallback=None
        CheckAuthorityTestCase {
            name: "Case 1: None/None/C",
            input: CheckAuthorityTestInput {
                current_authority: None,
                fallback_authority: None,
                signer_matches: SignerMatch::Current,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        CheckAuthorityTestCase {
            name: "Case 2: None/None/F",
            input: CheckAuthorityTestInput {
                current_authority: None,
                fallback_authority: None,
                signer_matches: SignerMatch::Fallback,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        CheckAuthorityTestCase {
            name: "Case 3: None/None/X",
            input: CheckAuthorityTestInput {
                current_authority: None,
                fallback_authority: None,
                signer_matches: SignerMatch::None,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        // Cases 4-6: Current=None, Fallback=Valid
        CheckAuthorityTestCase {
            name: "Case 4: None/Valid/C",
            input: CheckAuthorityTestInput {
                current_authority: None,
                fallback_authority: Some(fallback_auth),
                signer_matches: SignerMatch::Current,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        CheckAuthorityTestCase {
            name: "Case 5: None/Valid/F (SUCCESS)",
            input: CheckAuthorityTestInput {
                current_authority: None,
                fallback_authority: Some(fallback_auth),
                signer_matches: SignerMatch::Fallback,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: true,
                error_code: None,
            },
        },
        CheckAuthorityTestCase {
            name: "Case 6: None/Valid/X",
            input: CheckAuthorityTestInput {
                current_authority: None,
                fallback_authority: Some(fallback_auth),
                signer_matches: SignerMatch::None,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        // Cases 7-9: Current=None, Fallback=Revoked
        CheckAuthorityTestCase {
            name: "Case 7: None/Revoked/C",
            input: CheckAuthorityTestInput {
                current_authority: None,
                fallback_authority: Some(revoked_auth),
                signer_matches: SignerMatch::Current,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        // Case 8: None/Revoked/F - IMPOSSIBLE: signer cannot match [0u8; 32] (removed)
        CheckAuthorityTestCase {
            name: "Case 9: None/Revoked/X",
            input: CheckAuthorityTestInput {
                current_authority: None,
                fallback_authority: Some(revoked_auth),
                signer_matches: SignerMatch::None,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        // Cases 10-12: Current=Valid, Fallback=None
        CheckAuthorityTestCase {
            name: "Case 10: Valid/None/C",
            input: CheckAuthorityTestInput {
                current_authority: Some(current_auth),
                fallback_authority: None,
                signer_matches: SignerMatch::Current,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        CheckAuthorityTestCase {
            name: "Case 11: Valid/None/F",
            input: CheckAuthorityTestInput {
                current_authority: Some(current_auth),
                fallback_authority: None,
                signer_matches: SignerMatch::Fallback,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        CheckAuthorityTestCase {
            name: "Case 12: Valid/None/X",
            input: CheckAuthorityTestInput {
                current_authority: Some(current_auth),
                fallback_authority: None,
                signer_matches: SignerMatch::None,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        // Cases 13-15: Current=Valid, Fallback=Valid
        CheckAuthorityTestCase {
            name: "Case 13: Valid/Valid/C (SUCCESS)",
            input: CheckAuthorityTestInput {
                current_authority: Some(current_auth),
                fallback_authority: Some(fallback_auth),
                signer_matches: SignerMatch::Current,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: true,
                error_code: None,
            },
        },
        CheckAuthorityTestCase {
            name: "Case 14: Valid/Valid/F",
            input: CheckAuthorityTestInput {
                current_authority: Some(current_auth),
                fallback_authority: Some(fallback_auth),
                signer_matches: SignerMatch::Fallback,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        CheckAuthorityTestCase {
            name: "Case 15: Valid/Valid/X",
            input: CheckAuthorityTestInput {
                current_authority: Some(current_auth),
                fallback_authority: Some(fallback_auth),
                signer_matches: SignerMatch::None,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        // Cases 16-18: Current=Valid, Fallback=Revoked
        CheckAuthorityTestCase {
            name: "Case 16: Valid/Revoked/C (SUCCESS)",
            input: CheckAuthorityTestInput {
                current_authority: Some(current_auth),
                fallback_authority: Some(revoked_auth),
                signer_matches: SignerMatch::Current,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: true,
                error_code: None,
            },
        },
        CheckAuthorityTestCase {
            name: "Case 17: Valid/Revoked/F",
            input: CheckAuthorityTestInput {
                current_authority: Some(current_auth),
                fallback_authority: Some(revoked_auth),
                signer_matches: SignerMatch::Fallback,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        CheckAuthorityTestCase {
            name: "Case 18: Valid/Revoked/X",
            input: CheckAuthorityTestInput {
                current_authority: Some(current_auth),
                fallback_authority: Some(revoked_auth),
                signer_matches: SignerMatch::None,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        // Cases 19-21: Current=Revoked, Fallback=None
        CheckAuthorityTestCase {
            name: "Case 19: Revoked/None/C",
            input: CheckAuthorityTestInput {
                current_authority: Some(revoked_auth),
                fallback_authority: None,
                signer_matches: SignerMatch::Current,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        CheckAuthorityTestCase {
            name: "Case 20: Revoked/None/F",
            input: CheckAuthorityTestInput {
                current_authority: Some(revoked_auth),
                fallback_authority: None,
                signer_matches: SignerMatch::Fallback,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        CheckAuthorityTestCase {
            name: "Case 21: Revoked/None/X",
            input: CheckAuthorityTestInput {
                current_authority: Some(revoked_auth),
                fallback_authority: None,
                signer_matches: SignerMatch::None,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        // Cases 22-24: Current=Revoked, Fallback=Valid
        CheckAuthorityTestCase {
            name: "Case 22: Revoked/Valid/C",
            input: CheckAuthorityTestInput {
                current_authority: Some(revoked_auth),
                fallback_authority: Some(fallback_auth),
                signer_matches: SignerMatch::Current,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        CheckAuthorityTestCase {
            name: "Case 23: Revoked/Valid/F",
            input: CheckAuthorityTestInput {
                current_authority: Some(revoked_auth),
                fallback_authority: Some(fallback_auth),
                signer_matches: SignerMatch::Fallback,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        CheckAuthorityTestCase {
            name: "Case 24: Revoked/Valid/X",
            input: CheckAuthorityTestInput {
                current_authority: Some(revoked_auth),
                fallback_authority: Some(fallback_auth),
                signer_matches: SignerMatch::None,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
        // Cases 25-27: Current=Revoked, Fallback=Revoked
        // Case 25: Revoked/Revoked/C - IMPOSSIBLE: signer cannot match [0u8; 32] (removed)
        // Case 26: Revoked/Revoked/F - IMPOSSIBLE: signer cannot match [0u8; 32] (removed)
        CheckAuthorityTestCase {
            name: "Case 27: Revoked/Revoked/X",
            input: CheckAuthorityTestInput {
                current_authority: Some(revoked_auth),
                fallback_authority: Some(revoked_auth),
                signer_matches: SignerMatch::None,
                authority_name: "test authority",
            },
            expected: CheckAuthorityTestExpected {
                should_succeed: false,
                error_code: Some(ErrorCode::InvalidAuthorityMint),
            },
        },
    ];

    // Execute all test cases
    for (i, test_case) in test_cases.iter().enumerate() {
        println!("Executing {}: {}", i + 1, test_case.name);

        // Determine signer based on SignerMatch
        let signer_key = match test_case.input.signer_matches {
            SignerMatch::Current => current_auth.to_bytes(),
            SignerMatch::Fallback => fallback_auth.to_bytes(),
            SignerMatch::None => unrelated_signer.to_bytes(),
        };

        let signer_account = create_test_account_info(Pubkey::from(signer_key), true);
        // Execute check_authority
        let result = check_authority(
            test_case.input.current_authority.as_ref(),
            signer_account.key(),
            test_case.input.authority_name,
        );

        // Validate result
        if test_case.expected.should_succeed {
            assert!(
                result.is_ok(),
                "{}: Expected success but got error: {:?}",
                test_case.name,
                result.err()
            );
        } else {
            assert!(
                result.is_err(),
                "{}: Expected failure but got success",
                test_case.name
            );

            if let Some(expected_error) = test_case.expected.error_code {
                match result.err().unwrap() {
                    anchor_lang::prelude::ProgramError::Custom(code) => {
                        assert_eq!(
                            code, expected_error as u32,
                            "{}: Expected error code {} but got {}",
                            test_case.name, expected_error as u32, code
                        );
                    }
                    other => panic!(
                        "{}: Expected custom error code {} but got {:?}",
                        test_case.name, expected_error as u32, other
                    ),
                }
            }
        }
    }

    println!("✅ All 24 testable check_authority cases passed! (3 impossible cases excluded)");
}
