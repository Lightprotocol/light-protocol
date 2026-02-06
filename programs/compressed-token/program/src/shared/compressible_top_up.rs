use anchor_lang::solana_program::program_error::ProgramError;
use light_program_profiler::profile;
#[cfg(target_os = "solana")]
use light_token_interface::state::{
    mint_top_up_lamports_from_account_info, top_up_lamports_from_account_info_unchecked,
};
use light_token_interface::TokenError;
use pinocchio::{
    account_info::AccountInfo,
    sysvars::{clock::Clock, Sysvar},
};

use super::{
    convert_program_error,
    transfer_lamports::{multi_transfer_lamports, Transfer},
};

/// Calculate and execute top-up transfers for compressible CMint and CToken accounts.
/// CMint always has compression info. CToken requires Compressible extension or errors.
///
/// # Arguments
/// * `cmint` - The CMint account (may or may not have Compressible extension)
/// * `ctoken` - The CToken account (may or may not have Compressible extension)
/// * `payer` - The fee payer for top-ups
/// * `max_top_up` - Maximum lamports for top-ups combined (0 = no limit)
#[inline(always)]
#[profile]
#[allow(unused)]
pub fn calculate_and_execute_compressible_top_ups<'a>(
    cmint: &'a AccountInfo,
    ctoken: &'a AccountInfo,
    payer: Option<&'a AccountInfo>,
    max_top_up: u16,
) -> Result<(), ProgramError> {
    let mut transfers = [
        Transfer {
            account: cmint,
            amount: 0,
        },
        Transfer {
            account: ctoken,
            amount: 0,
        },
    ];

    let mut current_slot = 0;

    // Initialize budget: +1 allows exact match (total == max_top_up)
    let mut lamports_budget = (max_top_up as u64).saturating_add(1);

    // Calculate CMint top-up using optimized function (owner check inside)
    #[cfg(target_os = "solana")]
    if let Some(amount) = mint_top_up_lamports_from_account_info(cmint, &mut current_slot) {
        transfers[0].amount = amount;
        lamports_budget = lamports_budget.saturating_sub(amount);
    }

    // Calculate CToken top-up using optimized function
    // Returns None if no Compressible extension (165 bytes or missing extension)
    #[cfg(target_os = "solana")]
    if let Some(amount) = top_up_lamports_from_account_info_unchecked(ctoken, &mut current_slot) {
        transfers[1].amount = amount;
        lamports_budget = lamports_budget.saturating_sub(amount);
    }

    // Exit early if no compressible accounts (current_slot remains 0 if no top-ups calculated)
    if current_slot == 0 {
        return Ok(());
    }

    if transfers[0].amount == 0 && transfers[1].amount == 0 {
        return Ok(());
    }

    // Check budget wasn't exhausted (0 means exceeded max_top_up)
    if max_top_up != 0 && lamports_budget == 0 {
        return Err(TokenError::MaxTopUpExceeded.into());
    }
    let payer = payer.ok_or(TokenError::MissingPayer)?;
    multi_transfer_lamports(payer, &transfers).map_err(convert_program_error)?;
    Ok(())
}

/// Process compression top-up using embedded compression info.
/// Uses stored rent_exemption_paid from CompressionInfo instead of querying Rent sysvar.
#[inline(always)]
pub fn process_compression_top_up(
    compression: &light_compressible::compression_info::ZCompressionInfoMut<'_>,
    account_info: &AccountInfo,
    current_slot: &mut u64,
    transfer_amount: &mut u64,
    lamports_budget: &mut u64,
) -> Result<(), ProgramError> {
    if *current_slot == 0 {
        *current_slot = Clock::get()
            .map_err(|_| TokenError::SysvarAccessError)?
            .slot;
    }

    let previous_amount = *transfer_amount;
    *transfer_amount = compression
        .calculate_top_up_lamports(
            account_info.data_len() as u64,
            *current_slot,
            account_info.lamports(),
        )
        .map_err(|_| TokenError::InvalidAccountData)?;

    // Only deduct the delta from budget to avoid double-charging when
    // multiple compressions target the same account.
    let delta = transfer_amount.saturating_sub(previous_amount);
    *lamports_budget = lamports_budget.saturating_sub(delta);

    Ok(())
}
