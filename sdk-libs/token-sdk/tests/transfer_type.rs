//! Tests for transfer type determination based on account owners.

use light_token::{
    constants::LIGHT_TOKEN_PROGRAM_ID,
    error::TokenSdkError,
    instruction::{SplInterface, TransferInterface},
    utils::is_light_token_owner,
};
use solana_pubkey::Pubkey;

// SPL Token Program IDs (from light_token_types)
const SPL_TOKEN_PROGRAM_ID: Pubkey =
    Pubkey::new_from_array(light_token_types::SPL_TOKEN_PROGRAM_ID);
const SPL_TOKEN_2022_PROGRAM_ID: Pubkey =
    Pubkey::new_from_array(light_token_types::SPL_TOKEN_2022_PROGRAM_ID);

/// Verify is_light_token_owner returns Ok(true) for LIGHT_TOKEN_PROGRAM_ID.
#[test]
fn test_is_light_token_owner_light_program() {
    let result = is_light_token_owner(&LIGHT_TOKEN_PROGRAM_ID);

    assert!(
        result.is_ok(),
        "Should successfully identify Light token program"
    );
    assert!(result.unwrap(), "LIGHT_TOKEN_PROGRAM_ID should return true");
}

/// Verify is_light_token_owner returns Ok(false) for SPL_TOKEN_PROGRAM_ID.
#[test]
fn test_is_light_token_owner_spl_token() {
    let result = is_light_token_owner(&SPL_TOKEN_PROGRAM_ID);

    assert!(
        result.is_ok(),
        "Should successfully identify SPL token program"
    );
    assert!(!result.unwrap(), "SPL_TOKEN_PROGRAM_ID should return false");
}

/// Verify is_light_token_owner returns Ok(false) for SPL_TOKEN_2022_PROGRAM_ID.
#[test]
fn test_is_light_token_owner_spl_token_2022() {
    let result = is_light_token_owner(&SPL_TOKEN_2022_PROGRAM_ID);

    assert!(
        result.is_ok(),
        "Should successfully identify SPL Token 2022 program"
    );
    assert!(
        !result.unwrap(),
        "SPL_TOKEN_2022_PROGRAM_ID should return false"
    );
}

/// Verify is_light_token_owner returns Err for random/unknown program.
#[test]
fn test_is_light_token_owner_unknown_program() {
    let unknown_program = Pubkey::new_unique();
    let result = is_light_token_owner(&unknown_program);

    assert!(result.is_err(), "Unknown program should return error");
    match result {
        Err(TokenSdkError::CannotDetermineAccountType) => {
            // Expected error
        }
        Err(other) => {
            panic!("Expected CannotDetermineAccountType, got {:?}", other);
        }
        Ok(_) => {
            panic!("Expected error for unknown program");
        }
    }
}

/// Verify is_light_token_owner returns Err for system program.
#[test]
fn test_is_light_token_owner_system_program() {
    // System program ID (all zeros)
    let system_program = Pubkey::default();
    let result = is_light_token_owner(&system_program);

    assert!(result.is_err(), "System program should return error");
    match result {
        Err(TokenSdkError::CannotDetermineAccountType) => {
            // Expected error
        }
        Err(other) => {
            panic!("Expected CannotDetermineAccountType, got {:?}", other);
        }
        Ok(_) => {
            panic!("Expected error for system program");
        }
    }
}

/// Verify TransferInterface with both owners as LIGHT_TOKEN_PROGRAM_ID
/// does not require spl_interface (light-to-light transfer).
#[test]
fn test_transfer_interface_light_to_light_no_spl_interface() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let payer = Pubkey::new_unique();

    // Create TransferInterface for light-to-light transfer
    let transfer = TransferInterface {
        source,
        destination,
        amount: 1000,
        decimals: 9,
        authority,
        payer,
        spl_interface: None, // No SPL interface needed
        source_owner: LIGHT_TOKEN_PROGRAM_ID,
        destination_owner: LIGHT_TOKEN_PROGRAM_ID,
    };

    // Should succeed without spl_interface
    let result = transfer.instruction();
    assert!(
        result.is_ok(),
        "Light-to-light transfer should not require spl_interface: {:?}",
        result.err()
    );

    let instruction = result.unwrap();
    // Verify it's directed to the Light Token program
    assert_eq!(instruction.program_id, LIGHT_TOKEN_PROGRAM_ID);
}

/// Verify TransferInterface light-to-SPL requires spl_interface.
#[test]
fn test_transfer_interface_light_to_spl_requires_interface() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let payer = Pubkey::new_unique();

    // Create TransferInterface for light-to-SPL transfer without interface
    let transfer = TransferInterface {
        source,
        destination,
        amount: 1000,
        decimals: 9,
        authority,
        payer,
        spl_interface: None, // Missing required interface
        source_owner: LIGHT_TOKEN_PROGRAM_ID,
        destination_owner: SPL_TOKEN_PROGRAM_ID,
    };

    // Should fail without spl_interface
    let result = transfer.instruction();
    assert!(
        result.is_err(),
        "Light-to-SPL transfer should require spl_interface"
    );
}

/// Verify TransferInterface SPL-to-light requires spl_interface.
#[test]
fn test_transfer_interface_spl_to_light_requires_interface() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let payer = Pubkey::new_unique();

    // Create TransferInterface for SPL-to-light transfer without interface
    let transfer = TransferInterface {
        source,
        destination,
        amount: 1000,
        decimals: 9,
        authority,
        payer,
        spl_interface: None, // Missing required interface
        source_owner: SPL_TOKEN_PROGRAM_ID,
        destination_owner: LIGHT_TOKEN_PROGRAM_ID,
    };

    // Should fail without spl_interface
    let result = transfer.instruction();
    assert!(
        result.is_err(),
        "SPL-to-light transfer should require spl_interface"
    );
}

/// Verify TransferInterface light-to-SPL succeeds with spl_interface.
#[test]
fn test_transfer_interface_light_to_spl_with_interface() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let payer = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let spl_interface_pda = Pubkey::new_unique();

    let transfer = TransferInterface {
        source,
        destination,
        amount: 1000,
        decimals: 9,
        authority,
        payer,
        spl_interface: Some(SplInterface {
            mint,
            spl_token_program: SPL_TOKEN_PROGRAM_ID,
            spl_interface_pda,
            spl_interface_pda_bump: 255,
        }),
        source_owner: LIGHT_TOKEN_PROGRAM_ID,
        destination_owner: SPL_TOKEN_PROGRAM_ID,
    };

    let result = transfer.instruction();
    assert!(
        result.is_ok(),
        "Light-to-SPL transfer with spl_interface should succeed: {:?}",
        result.err()
    );
}

/// Verify TransferInterface SPL-to-SPL also requires spl_interface.
#[test]
fn test_transfer_interface_spl_to_spl_requires_interface() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let payer = Pubkey::new_unique();

    // Both owners are the same SPL token program
    let transfer = TransferInterface {
        source,
        destination,
        amount: 1000,
        decimals: 9,
        authority,
        payer,
        spl_interface: None, // Missing interface
        source_owner: SPL_TOKEN_PROGRAM_ID,
        destination_owner: SPL_TOKEN_PROGRAM_ID,
    };

    // SPL-to-SPL still goes through TransferInterface and needs mint info
    let result = transfer.instruction();
    assert!(
        result.is_err(),
        "SPL-to-SPL transfer through TransferInterface should require spl_interface for mint"
    );
}

/// Verify TransferInterface fails when source and destination have different SPL programs.
#[test]
fn test_transfer_interface_spl_program_mismatch() {
    let source = Pubkey::new_unique();
    let destination = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let payer = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let spl_interface_pda = Pubkey::new_unique();

    // Source is SPL Token, destination is SPL Token 2022
    let transfer = TransferInterface {
        source,
        destination,
        amount: 1000,
        decimals: 9,
        authority,
        payer,
        spl_interface: Some(SplInterface {
            mint,
            spl_token_program: SPL_TOKEN_PROGRAM_ID,
            spl_interface_pda,
            spl_interface_pda_bump: 255,
        }),
        source_owner: SPL_TOKEN_PROGRAM_ID,
        destination_owner: SPL_TOKEN_2022_PROGRAM_ID,
    };

    // Should fail due to program mismatch
    let result = transfer.instruction();
    assert!(
        result.is_err(),
        "Transfer between different SPL programs should fail"
    );
}

/// Verify known program ID values match expected strings.
#[test]
fn test_program_id_values() {
    // LIGHT_TOKEN_PROGRAM_ID = "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"
    assert_eq!(
        LIGHT_TOKEN_PROGRAM_ID.to_string(),
        "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"
    );

    // SPL_TOKEN_PROGRAM_ID = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    assert_eq!(
        SPL_TOKEN_PROGRAM_ID.to_string(),
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    );

    // SPL_TOKEN_2022_PROGRAM_ID = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
    assert_eq!(
        SPL_TOKEN_2022_PROGRAM_ID.to_string(),
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
    );
}
