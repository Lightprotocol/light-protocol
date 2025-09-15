use std::panic::catch_unwind;

use light_account_checks::error::AccountError;
use light_system_program_pinocchio::{
    accounts::account_traits::{InvokeAccounts, SignerAccounts},
    invoke::instruction::InvokeInstruction,
};
use pinocchio::program_error::ProgramError;

// Import the account info getters from the invoke_cpi_instruction test file
mod invoke_cpi_instruction;
use invoke_cpi_instruction::{
    get_account_compression_authority_account_info, get_account_compression_program_account_info,
    get_authority_account_info, get_fee_payer_account_info, get_mut_account_info,
    get_non_executable_account_compression_program_account_info, get_noop_program_account_info,
    get_registered_program_pda_account_info, get_self_program_account_info,
    get_system_program_account_info,
};
#[test]
fn functional_from_account_infos() {
    let fee_payer = get_fee_payer_account_info();
    let authority = get_authority_account_info();
    let registered_program_pda = get_registered_program_pda_account_info();
    let noop_program = get_noop_program_account_info();
    let account_compression_authority = get_account_compression_authority_account_info();
    let account_compression_program = get_account_compression_program_account_info();
    let sol_pool_pda_none = get_self_program_account_info();
    let system_program = get_system_program_account_info();
    let decompression_recipient = get_self_program_account_info();

    let ref_invoke_cpi_instruction = InvokeInstruction {
        fee_payer: &fee_payer,
        authority: &authority,
        registered_program_pda: &registered_program_pda,
        account_compression_authority: &account_compression_authority,
        account_compression_program: &account_compression_program,
        sol_pool_pda: None,
        decompression_recipient: None,
        system_program: &system_program,
    };
    let account_info_array = [
        fee_payer,
        authority,
        registered_program_pda,
        noop_program,
        account_compression_authority,
        account_compression_program,
        sol_pool_pda_none,
        decompression_recipient,
        system_program,
    ];
    let (invoke_cpi_instruction, _) =
        InvokeInstruction::from_account_infos(account_info_array.as_slice()).unwrap();
    assert!(invoke_cpi_instruction == ref_invoke_cpi_instruction);
    assert_eq!(
        invoke_cpi_instruction.get_fee_payer().key(),
        fee_payer.key()
    );
    assert_eq!(
        invoke_cpi_instruction.get_authority().key(),
        authority.key()
    );
    assert_eq!(
        invoke_cpi_instruction
            .get_account_compression_authority()
            .unwrap()
            .key(),
        account_compression_authority.key()
    );
    assert_eq!(
        invoke_cpi_instruction
            .get_registered_program_pda()
            .unwrap()
            .key(),
        registered_program_pda.key()
    );
    assert!(invoke_cpi_instruction.get_sol_pool_pda().unwrap().is_none());
    assert!(invoke_cpi_instruction
        .get_decompression_recipient()
        .unwrap()
        .is_none());
}

#[test]
fn failing_from_account_infos() {
    let fee_payer = get_fee_payer_account_info();
    let authority = get_authority_account_info();
    let registered_program_pda = get_registered_program_pda_account_info();
    let noop_program = get_noop_program_account_info();
    let account_compression_authority = get_account_compression_authority_account_info();
    let account_compression_program = get_account_compression_program_account_info();
    let sol_pool_pda_none = get_self_program_account_info();
    let system_program = get_system_program_account_info();
    let decompression_recipient = get_self_program_account_info();

    let ref_invoke_cpi_instruction = InvokeInstruction {
        fee_payer: &fee_payer,
        authority: &authority,
        registered_program_pda: &registered_program_pda,
        account_compression_authority: &account_compression_authority,
        account_compression_program: &account_compression_program,
        sol_pool_pda: None,
        decompression_recipient: None,
        system_program: &system_program,
    };
    let account_info_array = [
        fee_payer,
        authority,
        registered_program_pda,
        noop_program,
        account_compression_authority,
        account_compression_program,
        sol_pool_pda_none,
        decompression_recipient,
        system_program,
    ];
    // 1. Functional
    {
        let (invoke_cpi_instruction, _) =
            InvokeInstruction::from_account_infos(account_info_array.as_slice()).unwrap();
        assert!(invoke_cpi_instruction == ref_invoke_cpi_instruction);
    }
    // 3. Registered Program Pda mutable
    {
        let mut account_info_array = account_info_array;
        account_info_array[2] = get_mut_account_info();
        let res = InvokeInstruction::from_account_infos(account_info_array.as_slice());
        assert!(res == Err(ProgramError::from(AccountError::AccountMutable)));
    }
    // 4. account_compression_authority mutable
    {
        let mut account_info_array = account_info_array;
        account_info_array[4] = get_mut_account_info();
        let res = InvokeInstruction::from_account_infos(account_info_array.as_slice());
        assert!(res == Err(ProgramError::from(AccountError::AccountMutable)));
    }
    // 5. account_compression_program invalid program id
    {
        let mut account_info_array = account_info_array;
        account_info_array[5] = get_mut_account_info();
        let res = InvokeInstruction::from_account_infos(account_info_array.as_slice());
        assert!(res == Err(ProgramError::from(AccountError::InvalidProgramId)));
    }
    // 6. account_compression_program not executable
    {
        let mut account_info_array = account_info_array;
        account_info_array[5] = get_non_executable_account_compression_program_account_info();
        let res = InvokeInstruction::from_account_infos(account_info_array.as_slice());
        assert!(res == Err(ProgramError::from(AccountError::ProgramNotExecutable)));
    }
    // 7. sol_pool_pda invalid address
    {
        let mut account_info_array = account_info_array;
        account_info_array[6] = get_mut_account_info();
        // Panics with Unable to find a viable program address bump seed
        let result = catch_unwind(|| {
            // Call the function that is expected to panic
            InvokeInstruction::from_account_infos(account_info_array.as_slice()).unwrap();
        });
        assert!(
            result.is_err(),
            "Expected function to panic, but it did not."
        );
    }
    // 8. system_program invalid program id
    {
        let mut account_info_array = account_info_array;
        account_info_array[8] = get_mut_account_info();
        let res = InvokeInstruction::from_account_infos(account_info_array.as_slice());
        assert!(res == Err(ProgramError::from(AccountError::InvalidProgramId)));
    }
}
