use std::panic::catch_unwind;

use light_account_checks::{
    account_info::test_account_info::pinocchio::{get_account_info, pubkey_unique},
    error::AccountError,
};
use light_compressed_account::constants::ACCOUNT_COMPRESSION_PROGRAM_ID;
use light_system_program_pinocchio::invoke_cpi::instruction::InvokeCpiInstruction;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

pub fn get_fee_payer_account_info() -> AccountInfo {
    get_account_info(
        pubkey_unique(),
        Pubkey::default(),
        true,
        true,
        false,
        Vec::new(),
    )
}

pub fn get_authority_account_info() -> AccountInfo {
    get_account_info(
        pubkey_unique(),
        Pubkey::default(),
        true,
        false,
        false,
        Vec::new(),
    )
}

/// Random account info since it is not tested
pub fn get_registered_program_pda_account_info() -> AccountInfo {
    get_account_info(
        pubkey_unique(),
        ACCOUNT_COMPRESSION_PROGRAM_ID,
        false,
        false,
        false,
        Vec::new(),
    )
}

pub fn get_noop_program_account_info() -> AccountInfo {
    get_account_info(
        pubkey_unique(),
        pubkey_unique(),
        false,
        false,
        true,
        Vec::new(),
    )
}

pub fn get_account_compression_program_account_info() -> AccountInfo {
    get_account_info(
        ACCOUNT_COMPRESSION_PROGRAM_ID,
        pubkey_unique(),
        false,
        false,
        true,
        Vec::new(),
    )
}

pub fn get_non_executable_account_compression_program_account_info() -> AccountInfo {
    get_account_info(
        ACCOUNT_COMPRESSION_PROGRAM_ID,
        pubkey_unique(),
        false,
        false,
        false,
        Vec::new(),
    )
}

/// Random account info since it is not tested
pub fn get_account_compression_authority_account_info() -> AccountInfo {
    get_account_info(
        pubkey_unique(),
        pubkey_unique(),
        false,
        false,
        false,
        Vec::new(),
    )
}

/// Random account info executable is true.
pub fn get_program_account_info() -> AccountInfo {
    get_account_info(
        pubkey_unique(),
        pubkey_unique(),
        false,
        false,
        true,
        Vec::new(),
    )
}

/// Random account info mutable is true.
pub fn get_mut_account_info() -> AccountInfo {
    get_account_info(
        pubkey_unique(),
        pubkey_unique(),
        false,
        true,
        false,
        Vec::new(),
    )
}

/// Random account info executable is true.
pub fn get_system_program_account_info() -> AccountInfo {
    get_account_info(
        Pubkey::default(),
        pubkey_unique(),
        false,
        false,
        true,
        Vec::new(),
    )
}

/// Random account info
/// 1. key is crate::ID
/// 2. executable is true.
pub fn get_self_program_account_info() -> AccountInfo {
    get_account_info(
        light_system_program_pinocchio::ID,
        pubkey_unique(),
        false,
        false,
        true,
        Vec::new(),
    )
}

#[test]
fn functional_from_account_infos() {
    let fee_payer = get_fee_payer_account_info();
    let authority = get_authority_account_info();
    let registered_program_pda = get_registered_program_pda_account_info();
    let noop_program = get_noop_program_account_info();
    let account_compression_authority = get_account_compression_authority_account_info();
    let account_compression_program = get_account_compression_program_account_info();
    let invoking_program = get_program_account_info();
    let sol_pool_pda_none = get_self_program_account_info();
    let system_program = get_system_program_account_info();
    let cpi_context_account_info_none = get_self_program_account_info();
    let decompression_recipient = get_self_program_account_info();

    let ref_invoke_cpi_instruction = InvokeCpiInstruction {
        fee_payer: &fee_payer.clone(),
        authority: &authority.clone(),
        registered_program_pda: &registered_program_pda.clone(),
        account_compression_authority: &account_compression_authority.clone(),
        account_compression_program: &account_compression_program.clone(),
        invoking_program: &invoking_program.clone(),
        sol_pool_pda: None,
        decompression_recipient: None,
        cpi_context_account: None,
        system_program: &system_program.clone(),
    };
    let account_info_array = [
        fee_payer,
        authority,
        registered_program_pda,
        noop_program,
        account_compression_authority,
        account_compression_program,
        invoking_program,
        sol_pool_pda_none,
        decompression_recipient,
        system_program,
        cpi_context_account_info_none,
    ];
    let (invoke_cpi_instruction, _) =
        InvokeCpiInstruction::from_account_infos(account_info_array.as_slice()).unwrap();
    assert!(invoke_cpi_instruction == ref_invoke_cpi_instruction);
}

#[test]
fn failing_from_account_infos() {
    let fee_payer = get_fee_payer_account_info();
    let authority = get_authority_account_info();
    let registered_program_pda = get_registered_program_pda_account_info();
    let noop_program = get_noop_program_account_info();
    let account_compression_authority = get_account_compression_authority_account_info();
    let account_compression_program = get_account_compression_program_account_info();
    let invoking_program = get_program_account_info();
    let sol_pool_pda_none = get_self_program_account_info();
    let system_program = get_system_program_account_info();
    let cpi_context_account_info_none = get_self_program_account_info();
    let decompression_recipient = get_self_program_account_info();

    let ref_invoke_cpi_instruction = InvokeCpiInstruction {
        fee_payer: &fee_payer.clone(),
        authority: &authority.clone(),
        registered_program_pda: &registered_program_pda.clone(),
        account_compression_authority: &account_compression_authority.clone(),
        account_compression_program: &account_compression_program.clone(),
        invoking_program: &invoking_program.clone(),
        sol_pool_pda: None,
        decompression_recipient: None,
        cpi_context_account: None,
        system_program: &system_program.clone(),
    };
    let account_info_array = [
        fee_payer,
        authority,
        registered_program_pda,
        noop_program,
        account_compression_authority,
        account_compression_program,
        invoking_program,
        sol_pool_pda_none,
        decompression_recipient,
        system_program,
        cpi_context_account_info_none,
    ];
    // 1. Functional
    {
        let (invoke_cpi_instruction, _) =
            InvokeCpiInstruction::from_account_infos(account_info_array.as_slice()).unwrap();
        assert!(invoke_cpi_instruction == ref_invoke_cpi_instruction);
    }
    // 2. Authority mutable
    {
        let mut account_info_array = account_info_array;
        account_info_array[1] = get_fee_payer_account_info();
        let res = InvokeCpiInstruction::from_account_infos(account_info_array.as_slice());
        assert!(res == Err(ProgramError::from(AccountError::AccountMutable)));
    }
    // 3. Registered Program Pda mutable
    {
        let mut account_info_array = account_info_array;
        account_info_array[2] = get_mut_account_info();
        let res = InvokeCpiInstruction::from_account_infos(account_info_array.as_slice());
        assert!(res == Err(ProgramError::from(AccountError::AccountMutable)));
    }
    // 4. account_compression_authority mutable
    {
        let mut account_info_array = account_info_array;
        account_info_array[4] = get_mut_account_info();
        let res = InvokeCpiInstruction::from_account_infos(account_info_array.as_slice());
        assert!(res == Err(ProgramError::from(AccountError::AccountMutable)));
    }
    // 5. account_compression_program invalid program id
    {
        let mut account_info_array = account_info_array;
        account_info_array[5] = get_mut_account_info();
        let res = InvokeCpiInstruction::from_account_infos(account_info_array.as_slice());
        assert!(res == Err(ProgramError::from(AccountError::InvalidProgramId)));
    }
    // 6. account_compression_program not executable
    {
        let mut account_info_array = account_info_array;
        account_info_array[5] = get_non_executable_account_compression_program_account_info();
        let res = InvokeCpiInstruction::from_account_infos(account_info_array.as_slice());
        assert!(res == Err(ProgramError::from(AccountError::ProgramNotExecutable)));
    }
    // 7. sol_pool_pda invalid address
    {
        let mut account_info_array = account_info_array;
        account_info_array[7] = get_mut_account_info();
        // Panics with Unable to find a viable program address bump seed
        let result = catch_unwind(|| {
            // Call the function that is expected to panic
            InvokeCpiInstruction::from_account_infos(account_info_array.as_slice()).unwrap();
        });
        assert!(
            result.is_err(),
            "Expected function to panic, but it did not."
        );
    }
    // 8. system_program invalid program id
    {
        let mut account_info_array = account_info_array;
        account_info_array[9] = get_mut_account_info();
        let res = InvokeCpiInstruction::from_account_infos(account_info_array.as_slice());
        assert!(res == Err(ProgramError::from(AccountError::InvalidProgramId)));
    }
    // 9. cpi_context_account invalid address
    {
        let mut account_info_array = account_info_array;
        account_info_array[10] = get_mut_account_info();
        // Panics with Unable to find a viable program address bump seed
        let result = catch_unwind(|| {
            // Call the function that is expected to panic
            InvokeCpiInstruction::from_account_infos(account_info_array.as_slice()).unwrap();
        });
        assert!(
            result.is_err(),
            "Expected function to panic, but it did not."
        );
    }
}
