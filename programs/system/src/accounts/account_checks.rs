use std::panic::Location;

use light_account_checks::{
    checks::{
        check_discriminator, check_mut, check_non_mut, check_owner, check_pda_seeds,
        check_pda_seeds_with_bump, check_program, check_signer,
    },
    AccountIterator,
};
use light_compressed_account::{
    constants::ACCOUNT_COMPRESSION_PROGRAM_ID, instruction_data::traits::AccountOptions,
};
use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{
    cpi_context::state::ZCpiContextAccount2,
    processor::sol_compression::{SOL_POOL_PDA_BUMP, SOL_POOL_PDA_SEED},
    Result,
};

#[profile]
pub fn check_fee_payer(fee_payer: Option<&AccountInfo>) -> Result<&AccountInfo> {
    let fee_payer = fee_payer.ok_or(ProgramError::NotEnoughAccountKeys)?;
    check_signer(fee_payer).map_err(ProgramError::from)?;
    check_mut(fee_payer).map_err(ProgramError::from)?;
    Ok(fee_payer)
}

#[profile]
pub fn check_authority(authority: Option<&AccountInfo>) -> Result<&AccountInfo> {
    let authority = authority.ok_or(ProgramError::NotEnoughAccountKeys)?;
    check_signer(authority).map_err(ProgramError::from)?;
    check_non_mut(authority)?;
    Ok(authority)
}

#[profile]
pub fn check_non_mut_account_info(account_info: Option<&AccountInfo>) -> Result<&AccountInfo> {
    let account_info = account_info.ok_or(ProgramError::NotEnoughAccountKeys)?;
    check_non_mut(account_info)?;
    Ok(account_info)
}

#[profile]
pub fn check_account_compression_program(
    account_compression_program: Option<&AccountInfo>,
) -> Result<&AccountInfo> {
    let account_compression_program =
        account_compression_program.ok_or(ProgramError::NotEnoughAccountKeys)?;
    check_program(&ACCOUNT_COMPRESSION_PROGRAM_ID, account_compression_program)
        .map_err(ProgramError::from)?;
    Ok(account_compression_program)
}

#[profile]
pub fn check_anchor_option_sol_pool_pda(
    account_info: Option<&AccountInfo>,
) -> Result<Option<&AccountInfo>> {
    let option_sol_pool_pda = account_info.ok_or(ProgramError::NotEnoughAccountKeys)?;
    let sol_pool_pda = if *option_sol_pool_pda.key() == crate::ID {
        None
    } else {
        check_pda_seeds_with_bump(
            &[SOL_POOL_PDA_SEED, &[SOL_POOL_PDA_BUMP]][..],
            &crate::ID,
            option_sol_pool_pda,
        )?;
        check_mut(option_sol_pool_pda).map_err(ProgramError::from)?;
        Some(option_sol_pool_pda)
    };
    Ok(sol_pool_pda)
}

/// Processes account equivalent to anchor Accounts Option<AccountInfo>.
#[profile]
pub fn anchor_option_mut_account_info(
    account_info: Option<&AccountInfo>,
) -> Result<Option<&AccountInfo>> {
    let option_decompression_recipient = account_info.ok_or(ProgramError::NotEnoughAccountKeys)?;
    let decompression_recipient = if *option_decompression_recipient.key() == crate::ID {
        None
    } else {
        check_mut(option_decompression_recipient).map_err(ProgramError::from)?;
        Some(option_decompression_recipient)
    };
    Ok(decompression_recipient)
}

#[profile]
pub fn check_system_program(account_info: Option<&AccountInfo>) -> Result<&AccountInfo> {
    let account_info = account_info.ok_or(ProgramError::NotEnoughAccountKeys)?;
    check_program(&Pubkey::default(), account_info)?;
    Ok(account_info)
}

#[profile]
pub fn check_anchor_option_cpi_context_account(
    account_info: Option<&AccountInfo>,
) -> Result<Option<&AccountInfo>> {
    let option_cpi_context_account = account_info.ok_or(ProgramError::NotEnoughAccountKeys)?;
    let cpi_context_account = if *option_cpi_context_account.key() == crate::ID {
        None
    } else {
        {
            check_owner(&crate::ID, option_cpi_context_account)?;
            check_discriminator::<ZCpiContextAccount2>(
                option_cpi_context_account.try_borrow_data()?.as_ref(),
            )?;
        }
        Some(option_cpi_context_account)
    };
    Ok(cpi_context_account)
}

#[profile]
pub fn check_option_decompression_recipient<'a>(
    account_infos: &mut AccountIterator<'a, AccountInfo>,
    account_options: AccountOptions,
) -> Result<Option<&'a AccountInfo>> {
    let account = if account_options.decompression_recipient {
        let option_decompression_recipient =
            account_infos.next_account("decompression_recipient")?;
        check_mut(option_decompression_recipient).map_err(ProgramError::from)?;
        Some(option_decompression_recipient)
    } else {
        None
    };
    Ok(account)
}

#[track_caller]
#[profile]
pub fn check_option_cpi_context_account<'a>(
    account_infos: &mut AccountIterator<'a, AccountInfo>,
    account_options: AccountOptions,
) -> Result<Option<&'a AccountInfo>> {
    let account = if account_options.cpi_context_account {
        let account_info = account_infos.next_account("cpi_context")?;
        check_owner(&crate::ID, account_info).inspect_err(|_| {
            let location = Location::caller();
            solana_msg::msg!(
                "ERROR: check_owner {:?} owner: {:?} for cpi_context failed. {}:{}:{}",
                solana_pubkey::Pubkey::new_from_array(*account_info.key()),
                // SAFETY: owner() returns a valid pointer to a 32-byte aligned Pubkey
                solana_pubkey::Pubkey::new_from_array(*account_info.owner()),
                location.file(),
                location.line(),
                location.column()
            )
        })?;
        check_discriminator::<ZCpiContextAccount2>(account_info.try_borrow_data()?.as_ref())
            .inspect_err(|_| {
                let location = Location::caller();
                solana_msg::msg!(
                    "ERROR: check_discriminator for cpi_context failed. {}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                )
            })?;
        Some(account_info)
    } else {
        None
    };
    Ok(account)
}

#[profile]
pub fn check_option_sol_pool_pda<'a>(
    account_infos: &mut AccountIterator<'a, AccountInfo>,
    account_options: AccountOptions,
) -> Result<Option<&'a AccountInfo>> {
    let sol_pool_pda = if account_options.sol_pool_pda {
        let option_sol_pool_pda = account_infos.next_account("sol_pool_pda")?;
        check_pda_seeds(&[SOL_POOL_PDA_SEED], &crate::ID, option_sol_pool_pda)?;
        check_mut(option_sol_pool_pda).map_err(ProgramError::from)?;
        Some(option_sol_pool_pda)
    } else {
        None
    };
    Ok(sol_pool_pda)
}
