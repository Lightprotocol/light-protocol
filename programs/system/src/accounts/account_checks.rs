use light_account_checks::checks::{
    check_account_info_mut, check_discriminator, check_mut, check_non_mut, check_owner,
    check_pda_seeds, check_pda_seeds_with_bump, check_program, check_signer,
};
use light_compressed_account::{
    constants::ACCOUNT_COMPRESSION_PROGRAM_ID, instruction_data::traits::AccountOptions,
};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{
    invoke_cpi::account::CpiContextAccount, processor::sol_compression::SOL_POOL_PDA_SEED, Result,
};

pub fn check_fee_payer(fee_payer: Option<&AccountInfo>) -> Result<&AccountInfo> {
    let fee_payer = fee_payer.ok_or(ProgramError::NotEnoughAccountKeys)?;
    check_signer(fee_payer).map_err(ProgramError::from)?;
    check_mut(fee_payer).map_err(ProgramError::from)?;
    Ok(fee_payer)
}

pub fn check_authority(authority: Option<&AccountInfo>) -> Result<&AccountInfo> {
    let authority = authority.ok_or(ProgramError::NotEnoughAccountKeys)?;
    check_signer(authority).map_err(ProgramError::from)?;
    check_non_mut(authority)?;
    Ok(authority)
}

pub fn check_non_mut_account_info(account_info: Option<&AccountInfo>) -> Result<&AccountInfo> {
    let account_info = account_info.ok_or(ProgramError::NotEnoughAccountKeys)?;
    check_non_mut(account_info)?;
    Ok(account_info)
}

pub fn check_account_compression_program(
    account_compression_program: Option<&AccountInfo>,
) -> Result<&AccountInfo> {
    let account_compression_program =
        account_compression_program.ok_or(ProgramError::NotEnoughAccountKeys)?;
    check_program(&ACCOUNT_COMPRESSION_PROGRAM_ID, account_compression_program)
        .map_err(ProgramError::from)?;
    Ok(account_compression_program)
}

pub fn check_anchor_option_sol_pool_pda(
    account_info: Option<&AccountInfo>,
) -> Result<Option<&AccountInfo>> {
    let option_sol_pool_pda = account_info.ok_or(ProgramError::NotEnoughAccountKeys)?;
    let sol_pool_pda = if *option_sol_pool_pda.key() == crate::ID {
        None
    } else {
        check_pda_seeds_with_bump(
            &[SOL_POOL_PDA_SEED, &[255]][..],
            &crate::ID,
            option_sol_pool_pda,
        )?;
        Some(option_sol_pool_pda)
    };
    Ok(sol_pool_pda)
}

/// Processes account equivalent to anchor Accounts Option<AccountInfo>.
pub fn anchor_option_account_info(
    account_info: Option<&AccountInfo>,
) -> Result<Option<&AccountInfo>> {
    let option_decompression_recipient = account_info.ok_or(ProgramError::NotEnoughAccountKeys)?;
    let decompression_recipient = if *option_decompression_recipient.key() == crate::ID {
        None
    } else {
        Some(option_decompression_recipient)
    };
    Ok(decompression_recipient)
}

pub fn check_system_program(account_info: Option<&AccountInfo>) -> Result<&AccountInfo> {
    let account_info = account_info.ok_or(ProgramError::NotEnoughAccountKeys)?;
    check_program(&Pubkey::default(), account_info)?;
    Ok(account_info)
}

pub fn check_anchor_option_cpi_context_account(
    account_info: Option<&AccountInfo>,
) -> Result<Option<&AccountInfo>> {
    let option_cpi_context_account = account_info.ok_or(ProgramError::NotEnoughAccountKeys)?;
    let cpi_context_account = if *option_cpi_context_account.key() == crate::ID {
        None
    } else {
        {
            check_owner(&crate::ID, option_cpi_context_account)?;
            check_discriminator::<CpiContextAccount>(
                option_cpi_context_account.try_borrow_data()?.as_ref(),
            )?;
        }
        Some(option_cpi_context_account)
    };
    Ok(cpi_context_account)
}

pub fn check_option_decompression_recipient<'a, I>(
    account_infos: &mut I,
    account_options: AccountOptions,
) -> Result<Option<&'a AccountInfo>>
where
    I: Iterator<Item = &'a AccountInfo>,
{
    let account = if account_options.decompression_recipient {
        let option_decompression_recipient = account_infos
            .next()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        Some(option_decompression_recipient)
    } else {
        None
    };
    Ok(account)
}

pub fn check_option_cpi_context_account<'a, I>(
    account_infos: &mut I,
    account_options: AccountOptions,
) -> Result<Option<&'a AccountInfo>>
where
    I: Iterator<Item = &'a AccountInfo>,
{
    let account = if account_options.cpi_context_account {
        let account_info = account_infos
            .next()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        check_owner(&crate::ID, account_info)?;
        check_discriminator::<CpiContextAccount>(account_info.try_borrow_data()?.as_ref())?;
        Some(account_info)
    } else {
        None
    };
    Ok(account)
}

pub fn check_option_sol_pool_pda<'a, I>(
    account_infos: &mut I,
    account_options: AccountOptions,
) -> Result<Option<&'a AccountInfo>>
where
    I: Iterator<Item = &'a AccountInfo>,
{
    let sol_pool_pda = if account_options.sol_pool_pda {
        let option_sol_pool_pda = account_infos
            .next()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        check_pda_seeds(&[SOL_POOL_PDA_SEED], &crate::ID, option_sol_pool_pda)?;
        Some(option_sol_pool_pda)
    } else {
        None
    };
    Ok(sol_pool_pda)
}
