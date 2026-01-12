use anchor_lang::prelude::ProgramError;
use light_compressed_token::shared::initialize_ctoken_account::is_valid_mint;
use pinocchio::pubkey::Pubkey;

const SPL_TOKEN_ID: Pubkey = spl_token::ID.to_bytes();
const SPL_TOKEN_2022_ID: Pubkey = spl_token_2022::ID.to_bytes();
const LIGHT_TOKEN_PROGRAM_ID: Pubkey = light_token_interface::LIGHT_TOKEN_PROGRAM_ID;
const SYSTEM_PROGRAM_ID: Pubkey = [0u8; 32];
const RANDOM_PROGRAM_ID: Pubkey = [42u8; 32];

const ACCOUNT_TYPE_UNINITIALIZED: u8 = 0;
const ACCOUNT_TYPE_MINT: u8 = 1;
const ACCOUNT_TYPE_ACCOUNT: u8 = 2;
const ACCOUNT_TYPE_UNKNOWN: u8 = 3;

/// Owner types for testing
#[derive(Debug, Clone, Copy)]
enum Owner {
    SplToken,
    Token2022,
    CToken,
    SystemProgram,
    RandomProgram,
}

impl Owner {
    fn pubkey(&self) -> &Pubkey {
        match self {
            Owner::SplToken => &SPL_TOKEN_ID,
            Owner::Token2022 => &SPL_TOKEN_2022_ID,
            Owner::CToken => &LIGHT_TOKEN_PROGRAM_ID,
            Owner::SystemProgram => &SYSTEM_PROGRAM_ID,
            Owner::RandomProgram => &RANDOM_PROGRAM_ID,
        }
    }
}

/// Data configurations for testing
#[derive(Debug, Clone)]
enum MintData {
    Empty,
    TooSmall(usize),     // < 82 bytes
    ExactSplSize,        // 82 bytes (valid for all)
    BetweenSizes(usize), // 83-165 bytes
    WithAccountType(u8), // 166+ bytes with specific AccountType
}

impl MintData {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            MintData::Empty => vec![],
            MintData::TooSmall(size) => vec![0u8; *size],
            MintData::ExactSplSize => vec![0u8; 82],
            MintData::BetweenSizes(size) => vec![0u8; *size],
            MintData::WithAccountType(account_type) => {
                let mut data = vec![0u8; 170];
                data[165] = *account_type;
                data
            }
        }
    }
}

/// Expected result for a test case
#[derive(Debug, Clone, Copy, PartialEq)]
enum Expected {
    Valid,              // Ok(true)
    Invalid,            // Ok(false)
    IncorrectProgramId, // Err(IncorrectProgramId)
}

/// Test case definition
struct TestCase {
    owner: Owner,
    data: MintData,
    expected: Expected,
    description: &'static str,
}

fn run_test_case(tc: &TestCase) {
    let data = tc.data.to_bytes();
    let result = is_valid_mint(tc.owner.pubkey(), &data);

    match tc.expected {
        Expected::Valid => {
            assert!(
                result.as_ref().map(|v| *v).unwrap_or(false),
                "FAILED: {} - expected Ok(true), got {:?}",
                tc.description,
                result
            );
        }
        Expected::Invalid => {
            assert!(
                result.as_ref().map(|v| !*v).unwrap_or(false),
                "FAILED: {} - expected Ok(false), got {:?}",
                tc.description,
                result
            );
        }
        Expected::IncorrectProgramId => {
            assert!(
                result.as_ref().err() == Some(&ProgramError::IncorrectProgramId),
                "FAILED: {} - expected Err(IncorrectProgramId), got {:?}",
                tc.description,
                result
            );
        }
    }
}

/// Systematically test all owner x data combinations
#[test]
fn test_is_valid_mint_all_combinations() {
    let test_cases = vec![
        // =========================================================================
        // INVALID OWNERS - should always return Err(IncorrectProgramId)
        // =========================================================================
        TestCase {
            owner: Owner::SystemProgram,
            data: MintData::ExactSplSize,
            expected: Expected::IncorrectProgramId,
            description: "System program owner with 82 bytes",
        },
        TestCase {
            owner: Owner::RandomProgram,
            data: MintData::ExactSplSize,
            expected: Expected::IncorrectProgramId,
            description: "Random program owner with 82 bytes",
        },
        TestCase {
            owner: Owner::SystemProgram,
            data: MintData::WithAccountType(ACCOUNT_TYPE_MINT),
            expected: Expected::IncorrectProgramId,
            description: "System program owner with AccountType=Mint",
        },
        // =========================================================================
        // SPL TOKEN - only accepts exactly 82 bytes
        // =========================================================================
        TestCase {
            owner: Owner::SplToken,
            data: MintData::Empty,
            expected: Expected::Invalid,
            description: "SPL: empty data",
        },
        TestCase {
            owner: Owner::SplToken,
            data: MintData::TooSmall(40),
            expected: Expected::Invalid,
            description: "SPL: 40 bytes (< 82)",
        },
        TestCase {
            owner: Owner::SplToken,
            data: MintData::TooSmall(81),
            expected: Expected::Invalid,
            description: "SPL: 81 bytes (off by one)",
        },
        TestCase {
            owner: Owner::SplToken,
            data: MintData::ExactSplSize,
            expected: Expected::Valid,
            description: "SPL: exactly 82 bytes (valid mint)",
        },
        TestCase {
            owner: Owner::SplToken,
            data: MintData::BetweenSizes(83),
            expected: Expected::Invalid,
            description: "SPL: 83 bytes (off by one, too large)",
        },
        TestCase {
            owner: Owner::SplToken,
            data: MintData::BetweenSizes(165),
            expected: Expected::Invalid,
            description: "SPL: 165 bytes (token account size)",
        },
        TestCase {
            owner: Owner::SplToken,
            data: MintData::WithAccountType(ACCOUNT_TYPE_MINT),
            expected: Expected::Invalid,
            description: "SPL: 170 bytes with AccountType=Mint (SPL doesnt support extensions)",
        },
        TestCase {
            owner: Owner::SplToken,
            data: MintData::WithAccountType(ACCOUNT_TYPE_ACCOUNT),
            expected: Expected::Invalid,
            description: "SPL: 170 bytes with AccountType=Account",
        },
        // =========================================================================
        // TOKEN-2022 - accepts 82 bytes OR 166+ with AccountType=Mint
        // =========================================================================
        TestCase {
            owner: Owner::Token2022,
            data: MintData::Empty,
            expected: Expected::Invalid,
            description: "T22: empty data",
        },
        TestCase {
            owner: Owner::Token2022,
            data: MintData::TooSmall(40),
            expected: Expected::Invalid,
            description: "T22: 40 bytes (< 82)",
        },
        TestCase {
            owner: Owner::Token2022,
            data: MintData::TooSmall(81),
            expected: Expected::Invalid,
            description: "T22: 81 bytes (off by one)",
        },
        TestCase {
            owner: Owner::Token2022,
            data: MintData::ExactSplSize,
            expected: Expected::Valid,
            description: "T22: exactly 82 bytes (valid mint without extensions)",
        },
        TestCase {
            owner: Owner::Token2022,
            data: MintData::BetweenSizes(83),
            expected: Expected::Invalid,
            description: "T22: 83 bytes (invalid - between sizes)",
        },
        TestCase {
            owner: Owner::Token2022,
            data: MintData::BetweenSizes(165),
            expected: Expected::Invalid,
            description: "T22: 165 bytes (edge case - no AccountType marker)",
        },
        TestCase {
            owner: Owner::Token2022,
            data: MintData::WithAccountType(ACCOUNT_TYPE_UNINITIALIZED),
            expected: Expected::Invalid,
            description: "T22: 170 bytes with AccountType=0 (uninitialized)",
        },
        TestCase {
            owner: Owner::Token2022,
            data: MintData::WithAccountType(ACCOUNT_TYPE_MINT),
            expected: Expected::Valid,
            description: "T22: 170 bytes with AccountType=Mint (valid)",
        },
        TestCase {
            owner: Owner::Token2022,
            data: MintData::WithAccountType(ACCOUNT_TYPE_ACCOUNT),
            expected: Expected::Invalid,
            description: "T22: 170 bytes with AccountType=Account (token account)",
        },
        TestCase {
            owner: Owner::Token2022,
            data: MintData::WithAccountType(ACCOUNT_TYPE_UNKNOWN),
            expected: Expected::Invalid,
            description: "T22: 170 bytes with AccountType=3 (unknown)",
        },
        TestCase {
            owner: Owner::Token2022,
            data: MintData::WithAccountType(255),
            expected: Expected::Invalid,
            description: "T22: 170 bytes with AccountType=255 (invalid)",
        },
        // =========================================================================
        // CTOKEN - must always be >165 bytes with AccountType=Mint
        // =========================================================================
        TestCase {
            owner: Owner::CToken,
            data: MintData::Empty,
            expected: Expected::Invalid,
            description: "CToken: empty data",
        },
        TestCase {
            owner: Owner::CToken,
            data: MintData::TooSmall(40),
            expected: Expected::Invalid,
            description: "CToken: 40 bytes (< 82)",
        },
        TestCase {
            owner: Owner::CToken,
            data: MintData::TooSmall(81),
            expected: Expected::Invalid,
            description: "CToken: 81 bytes (off by one)",
        },
        TestCase {
            owner: Owner::CToken,
            data: MintData::ExactSplSize,
            expected: Expected::Invalid,
            description: "CToken: 82 bytes (invalid - CToken always has extensions)",
        },
        TestCase {
            owner: Owner::CToken,
            data: MintData::BetweenSizes(83),
            expected: Expected::Invalid,
            description: "CToken: 83 bytes (invalid - between sizes)",
        },
        TestCase {
            owner: Owner::CToken,
            data: MintData::BetweenSizes(165),
            expected: Expected::Invalid,
            description: "CToken: 165 bytes (edge case - no AccountType marker)",
        },
        TestCase {
            owner: Owner::CToken,
            data: MintData::WithAccountType(ACCOUNT_TYPE_UNINITIALIZED),
            expected: Expected::Invalid,
            description: "CToken: 170 bytes with AccountType=0 (uninitialized)",
        },
        TestCase {
            owner: Owner::CToken,
            data: MintData::WithAccountType(ACCOUNT_TYPE_MINT),
            expected: Expected::Valid,
            description: "CToken: 170 bytes with AccountType=Mint (valid)",
        },
        TestCase {
            owner: Owner::CToken,
            data: MintData::WithAccountType(ACCOUNT_TYPE_ACCOUNT),
            expected: Expected::Invalid,
            description: "CToken: 170 bytes with AccountType=Account (token account)",
        },
        TestCase {
            owner: Owner::CToken,
            data: MintData::WithAccountType(ACCOUNT_TYPE_UNKNOWN),
            expected: Expected::Invalid,
            description: "CToken: 170 bytes with AccountType=3 (unknown)",
        },
    ];

    println!(
        "\nRunning {} test cases for is_valid_mint:\n",
        test_cases.len()
    );

    let mut passed = 0;
    let mut failed = 0;

    for tc in &test_cases {
        print!("  {:60} ... ", tc.description);
        let data = tc.data.to_bytes();
        let result = is_valid_mint(tc.owner.pubkey(), &data);

        let success = match tc.expected {
            Expected::Valid => result.as_ref().map(|v| *v).unwrap_or(false),
            Expected::Invalid => result.as_ref().map(|v| !*v).unwrap_or(false),
            Expected::IncorrectProgramId => {
                result.as_ref().err() == Some(&ProgramError::IncorrectProgramId)
            }
        };

        if success {
            println!("ok");
            passed += 1;
        } else {
            println!("FAILED (got {:?})", result);
            failed += 1;
        }
    }

    println!("\nResults: {} passed, {} failed\n", passed, failed);

    // Now run assertions to fail the test if any failed
    for tc in &test_cases {
        run_test_case(tc);
    }
}
