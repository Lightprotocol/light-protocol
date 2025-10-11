use light_account_checks::{
    account_info::test_account_info::pinocchio::{get_account_info, pubkey_unique},
    error::AccountError,
};
use light_compressed_account::instruction_data::traits::AccountOptions;
use light_system_program_pinocchio::{
    invoke_cpi::instruction_v2::InvokeCpiInstructionV2, CPI_CONTEXT_ACCOUNT_2_DISCRIMINATOR,
};
// We'll avoid direct PDA validation as it's difficult in unit tests
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;

// Import the account info getters from the invoke_cpi_instruction test file
mod invoke_cpi_instruction;
use invoke_cpi_instruction::{
    get_account_compression_authority_account_info, get_account_compression_program_account_info,
    get_authority_account_info, get_fee_payer_account_info, get_mut_account_info,
    get_registered_program_pda_account_info, get_self_program_account_info,
    get_system_program_account_info,
};

// Helper function to get a valid cpi_context_account with correct discriminator
fn get_valid_cpi_context_account_info() -> AccountInfo {
    // Create a new account owned by the system program
    let program_id = light_system_program_pinocchio::ID;

    // Create data with the correct discriminator at the beginning
    let mut data = vec![0; 100]; // Extra space for the account data
    data[0..8].copy_from_slice(&CPI_CONTEXT_ACCOUNT_2_DISCRIMINATOR);

    get_account_info(
        pubkey_unique(), // Random pubkey
        program_id,      // Owned by the system program
        false,           // Not a signer
        true,            // Is writable
        false,           // Not executable
        data,            // Data with discriminator
    )
}

// Helper function to get a decompression recipient account
fn get_decompression_recipient_account_info() -> AccountInfo {
    // Create a regular account
    get_account_info(
        pubkey_unique(), // Random pubkey
        pubkey_unique(), // Random owner
        false,           // Not a signer
        true,            // Is writable
        false,           // Not executable
        vec![],          // Minimal data
    )
}

#[test]
fn functional_from_account_infos_v2() {
    let fee_payer = get_fee_payer_account_info();
    let authority = get_authority_account_info();
    let registered_program_pda = get_registered_program_pda_account_info();
    let account_compression_authority = get_account_compression_authority_account_info();
    let account_compression_program = get_account_compression_program_account_info();
    let system_program = get_system_program_account_info();

    // No optional accounts
    {
        let options_config = AccountOptions {
            sol_pool_pda: false,
            decompression_recipient: false,
            cpi_context_account: false,
            write_to_cpi_context: false,
        };

        let account_info_array = [
            fee_payer,
            authority,
            registered_program_pda,
            account_compression_authority,
            account_compression_program,
            system_program,
            get_mut_account_info(), // Dummy remaining account
            get_mut_account_info(), // Another dummy remaining account
        ];
        let result = InvokeCpiInstructionV2::from_account_infos(
            account_info_array.as_slice(),
            options_config,
        );

        // Verify result is Ok and contains the expected accounts
        let (invoke_cpi_instruction_v2, _) = result.unwrap();
        assert_eq!(invoke_cpi_instruction_v2.fee_payer.key(), fee_payer.key());
        assert_eq!(invoke_cpi_instruction_v2.authority.key(), authority.key());
        assert_eq!(
            invoke_cpi_instruction_v2
                .exec_accounts
                .as_ref()
                .unwrap()
                .registered_program_pda
                .key(),
            registered_program_pda.key()
        );
        assert_eq!(invoke_cpi_instruction_v2.fee_payer.key(), fee_payer.key());
        assert_eq!(invoke_cpi_instruction_v2.authority.key(), authority.key());
        assert_eq!(
            invoke_cpi_instruction_v2
                .exec_accounts
                .as_ref()
                .unwrap()
                .registered_program_pda
                .key(),
            registered_program_pda.key()
        );
        assert_eq!(
            invoke_cpi_instruction_v2
                .exec_accounts
                .as_ref()
                .unwrap()
                .account_compression_authority
                .key(),
            account_compression_authority.key()
        );
        assert!(invoke_cpi_instruction_v2
            .exec_accounts
            .as_ref()
            .unwrap()
            .sol_pool_pda
            .is_none());
        assert!(invoke_cpi_instruction_v2
            .exec_accounts
            .unwrap()
            .decompression_recipient
            .is_none());
        assert!(invoke_cpi_instruction_v2.cpi_context_account.is_none());
    }

    // 1. With decompression recipient
    {
        let decompression_recipient = get_decompression_recipient_account_info();
        let options_config = AccountOptions {
            sol_pool_pda: false,
            decompression_recipient: true,
            cpi_context_account: false,
            write_to_cpi_context: false, // TODO: test with write_to_cpi_context
        };

        let account_info_array = [
            fee_payer,
            authority,
            registered_program_pda,
            account_compression_authority,
            account_compression_program,
            system_program,
            decompression_recipient,
            get_mut_account_info(), // Remaining account required for CPI
        ];

        let result = InvokeCpiInstructionV2::from_account_infos(
            account_info_array.as_slice(),
            options_config,
        );

        let (invoke_cpi_instruction_v2, _) = result.unwrap();
        assert_eq!(invoke_cpi_instruction_v2.fee_payer.key(), fee_payer.key());
        assert_eq!(invoke_cpi_instruction_v2.authority.key(), authority.key());
        assert!(invoke_cpi_instruction_v2
            .exec_accounts
            .as_ref()
            .unwrap()
            .sol_pool_pda
            .is_none());
        assert_eq!(
            invoke_cpi_instruction_v2
                .exec_accounts
                .unwrap()
                .decompression_recipient
                .unwrap()
                .key(),
            decompression_recipient.key()
        );
        assert!(invoke_cpi_instruction_v2.cpi_context_account.is_none());
    }
    // With cpi_context_account
    {
        let fee_payer = get_fee_payer_account_info();
        let authority = get_authority_account_info();
        let registered_program_pda = get_registered_program_pda_account_info();
        let account_compression_authority = get_account_compression_authority_account_info();
        let account_compression_program = get_account_compression_program_account_info();
        let system_program = get_system_program_account_info();
        let cpi_context_account = get_valid_cpi_context_account_info();

        let options_config = AccountOptions {
            sol_pool_pda: false,
            decompression_recipient: false,
            cpi_context_account: true,
            write_to_cpi_context: false,
        };

        let account_info_array = [
            fee_payer,
            authority,
            registered_program_pda,
            account_compression_authority,
            account_compression_program,
            system_program,
            cpi_context_account,
            get_mut_account_info(), // Remaining account required for CPI
        ];

        // This should pass with valid discriminator
        let result = InvokeCpiInstructionV2::from_account_infos(
            account_info_array.as_slice(),
            options_config,
        );

        // Verify result is Ok and contains the expected accounts
        let (invoke_cpi_instruction_v2, _) = result.unwrap();
        assert_eq!(invoke_cpi_instruction_v2.fee_payer.key(), fee_payer.key());
        assert_eq!(invoke_cpi_instruction_v2.authority.key(), authority.key());
        assert!(invoke_cpi_instruction_v2
            .exec_accounts
            .as_ref()
            .unwrap()
            .sol_pool_pda
            .is_none());
        assert!(invoke_cpi_instruction_v2
            .exec_accounts
            .unwrap()
            .decompression_recipient
            .is_none());
        assert_eq!(
            invoke_cpi_instruction_v2.cpi_context_account.unwrap().key(),
            cpi_context_account.key()
        );
    }
}

/// Test for invalid CPI context account
#[test]
fn test_cpi_context_account_error_handling() {
    let fee_payer = get_fee_payer_account_info();
    let authority = get_authority_account_info();
    let registered_program_pda = get_registered_program_pda_account_info();
    let account_compression_authority = get_account_compression_authority_account_info();
    let options_config = AccountOptions {
        sol_pool_pda: false, // Avoid PDA validation
        decompression_recipient: false,
        cpi_context_account: true,
        write_to_cpi_context: false,
    };
    // Invalid program owner
    {
        let invalid_cpi_context_account = get_self_program_account_info();
        let account_compression_program = get_account_compression_program_account_info();
        let system_program = get_system_program_account_info();
        let account_info_array = [
            fee_payer,
            authority,
            registered_program_pda,
            account_compression_authority,
            account_compression_program,
            system_program,
            invalid_cpi_context_account,
            get_mut_account_info(), // Remaining account required for CPI
        ];

        let result = InvokeCpiInstructionV2::from_account_infos(
            account_info_array.as_slice(),
            options_config,
        );

        assert!(result == Err(ProgramError::from(AccountError::AccountOwnedByWrongProgram)));
    }
    // Invalid discriminator
    {
        let invalid_cpi_context_account = get_valid_cpi_context_account_info();
        invalid_cpi_context_account.try_borrow_mut_data().unwrap()[..8].copy_from_slice(&[0; 8]);
        let account_compression_program = get_account_compression_program_account_info();
        let system_program = get_system_program_account_info();
        let account_info_array = [
            fee_payer,
            authority,
            registered_program_pda,
            account_compression_authority,
            account_compression_program,
            system_program,
            invalid_cpi_context_account,
            get_mut_account_info(), // Remaining account required for CPI
        ];

        let result = InvokeCpiInstructionV2::from_account_infos(
            account_info_array.as_slice(),
            options_config,
        );
        assert!(result == Err(ProgramError::from(AccountError::InvalidDiscriminator)));
    }
}

/// Test for decompression_recipient and cpi_context_account together
/// without requiring PDA validation
#[test]
fn test_decompression_recipient_and_cpi_context_validation() {
    let fee_payer = get_fee_payer_account_info();
    let authority = get_authority_account_info();
    let registered_program_pda = get_registered_program_pda_account_info();
    let account_compression_authority = get_account_compression_authority_account_info();
    let decompression_recipient = get_decompression_recipient_account_info();
    let cpi_context_account = get_valid_cpi_context_account_info();

    let options_config = AccountOptions {
        sol_pool_pda: false,
        decompression_recipient: true,
        cpi_context_account: true,
        write_to_cpi_context: false,
    };

    let account_compression_program = get_account_compression_program_account_info();
    let system_program = get_system_program_account_info();

    let account_info_array = [
        fee_payer,
        authority,
        registered_program_pda,
        account_compression_authority,
        account_compression_program,
        system_program,
        decompression_recipient,
        cpi_context_account,
        get_mut_account_info(), // Remaining account required for CPI
    ];

    // This should pass with valid discriminator
    let result =
        InvokeCpiInstructionV2::from_account_infos(account_info_array.as_slice(), options_config);

    // Verify result is Ok and contains the expected accounts
    let (invoke_cpi_instruction_v2, _) = result.unwrap();
    assert_eq!(invoke_cpi_instruction_v2.fee_payer.key(), fee_payer.key());
    assert_eq!(invoke_cpi_instruction_v2.authority.key(), authority.key());
    assert!(invoke_cpi_instruction_v2
        .exec_accounts
        .as_ref()
        .unwrap()
        .sol_pool_pda
        .is_none());
    assert_eq!(
        invoke_cpi_instruction_v2
            .exec_accounts
            .unwrap()
            .decompression_recipient
            .unwrap()
            .key(),
        decompression_recipient.key()
    );
    assert_eq!(
        invoke_cpi_instruction_v2.cpi_context_account.unwrap().key(),
        cpi_context_account.key()
    );
}

#[test]
fn failing_from_account_infos_v2() {
    let fee_payer = get_fee_payer_account_info();
    let authority = get_authority_account_info();
    let registered_program_pda = get_registered_program_pda_account_info();
    let account_compression_authority = get_account_compression_authority_account_info();

    let account_compression_program = get_account_compression_program_account_info();
    let system_program = get_system_program_account_info();

    // Base array for tests
    let account_info_array = [
        fee_payer,
        authority,
        registered_program_pda,
        account_compression_authority,
        account_compression_program,
        system_program,
        get_mut_account_info(), // Remaining account required for CPI
    ];

    // 1. Functional test
    {
        let options_config = AccountOptions {
            sol_pool_pda: false,
            decompression_recipient: false,
            cpi_context_account: false,
            write_to_cpi_context: false,
        };

        let result = InvokeCpiInstructionV2::from_account_infos(
            account_info_array.as_slice(),
            options_config,
        );
        assert!(result.is_ok());
    }

    // 2. Authority mutable
    {
        let options_config = AccountOptions {
            sol_pool_pda: false,
            decompression_recipient: false,
            cpi_context_account: false,
            write_to_cpi_context: false,
        };

        let mut account_info_array_clone = account_info_array;
        account_info_array_clone[1] = get_fee_payer_account_info(); // Use a mutable account

        let result = InvokeCpiInstructionV2::from_account_infos(
            account_info_array_clone.as_slice(),
            options_config,
        );

        match result {
            Err(err) => assert_eq!(err, ProgramError::from(AccountError::AccountMutable)),
            Ok(_) => panic!("Expected an error for mutable authority but got Ok"),
        }
    }

    // 3. Registered Program Pda mutable
    {
        let options_config = AccountOptions {
            sol_pool_pda: false,
            decompression_recipient: false,
            cpi_context_account: false,
            write_to_cpi_context: false,
        };

        let mut account_info_array_clone = account_info_array;
        account_info_array_clone[2] = get_mut_account_info();

        let result = InvokeCpiInstructionV2::from_account_infos(
            account_info_array_clone.as_slice(),
            options_config,
        );

        match result {
            Err(err) => assert_eq!(err, ProgramError::from(AccountError::AccountMutable)),
            Ok(_) => panic!("Expected an error for mutable registered_program_pda but got Ok"),
        }
    }

    // 4. Account Compression Authority mutable
    {
        let options_config = AccountOptions {
            sol_pool_pda: false,
            decompression_recipient: false,
            cpi_context_account: false,
            write_to_cpi_context: false,
        };

        let mut account_info_array_clone = account_info_array;
        account_info_array_clone[3] = get_mut_account_info();

        let result = InvokeCpiInstructionV2::from_account_infos(
            account_info_array_clone.as_slice(),
            options_config,
        );

        match result {
            Err(err) => assert_eq!(err, ProgramError::from(AccountError::AccountMutable)),
            Ok(_) => {
                panic!("Expected an error for mutable account_compression_authority but got Ok")
            }
        }
    }

    // 5. Not enough accounts (missing required)
    {
        let options_config = AccountOptions {
            sol_pool_pda: false,
            decompression_recipient: false,
            cpi_context_account: false,
            write_to_cpi_context: false,
        };

        let insufficient_array = [
            fee_payer,
            authority,
            // Missing registered_program_pda and account_compression_authority
        ];

        // This will panic with index out of bounds
        let result = InvokeCpiInstructionV2::from_account_infos(
            insufficient_array.as_slice(),
            options_config,
        );

        assert!(result.is_err(), "Expected a panic due to missing accounts");
    }

    // 6. Test with optional accounts (with decompression_recipient and checking it's set correctly)
    {
        let decompression_recipient = get_decompression_recipient_account_info();
        let account_compression_program = get_account_compression_program_account_info();
        let system_program = get_system_program_account_info();
        let options_with_decompression = AccountOptions {
            sol_pool_pda: false,
            decompression_recipient: true,
            cpi_context_account: false,
            write_to_cpi_context: false,
        };

        let account_array_with_decompression = [
            fee_payer,
            authority,
            registered_program_pda,
            account_compression_authority,
            account_compression_program,
            system_program,
            decompression_recipient,
            get_mut_account_info(), // Remaining account required for CPI
        ];

        let result = InvokeCpiInstructionV2::from_account_infos(
            account_array_with_decompression.as_slice(),
            options_with_decompression,
        );

        // This should pass since it doesn't require PDA validation
        let (instruction, _) = result.unwrap();
        assert!(instruction
            .exec_accounts
            .as_ref()
            .unwrap()
            .sol_pool_pda
            .is_none());
        assert_eq!(
            instruction
                .exec_accounts
                .as_ref()
                .unwrap()
                .decompression_recipient
                .unwrap()
                .key(),
            decompression_recipient.key()
        );
        assert!(instruction.cpi_context_account.is_none());
    }
}
