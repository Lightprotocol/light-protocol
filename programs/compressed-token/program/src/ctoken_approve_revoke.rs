use anchor_lang::solana_program::program_error::ProgramError;
use light_ctoken_types::{state::CToken, BASE_TOKEN_ACCOUNT_SIZE};
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::{approve::process_approve, revoke::process_revoke};

use crate::{
    shared::{convert_program_error, transfer_lamports_via_cpi},
    transfer2::compression::ctoken::process_compressible_extension,
};

/// Account indices for approve instruction
const APPROVE_ACCOUNT_SOURCE: usize = 0;
const APPROVE_ACCOUNT_OWNER: usize = 2; // owner is payer for top-up

/// Account indices for revoke instruction
const REVOKE_ACCOUNT_SOURCE: usize = 0;
const REVOKE_ACCOUNT_OWNER: usize = 1; // owner is payer for top-up

/// Process CToken approve instruction.
/// Handles compressible extension top-up before delegating to pinocchio.
#[inline(always)]
pub fn process_ctoken_approve(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let source = accounts
        .get(APPROVE_ACCOUNT_SOURCE)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let payer = accounts
        .get(APPROVE_ACCOUNT_OWNER)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    // Handle compressible top-up before pinocchio call
    process_compressible_top_up(source, payer)?;

    process_approve(accounts, instruction_data).map_err(convert_program_error)
}

/// Process CToken revoke instruction.
/// Handles compressible extension top-up before delegating to pinocchio.
#[inline(always)]
pub fn process_ctoken_revoke(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
    let source = accounts
        .get(REVOKE_ACCOUNT_SOURCE)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let payer = accounts
        .get(REVOKE_ACCOUNT_OWNER)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    // Handle compressible top-up before pinocchio call
    process_compressible_top_up(source, payer)?;

    process_revoke(accounts).map_err(convert_program_error)
}

/// Calculate and transfer compressible top-up for a single account.
#[inline(always)]
fn process_compressible_top_up(
    account: &AccountInfo,
    payer: &AccountInfo,
) -> Result<(), ProgramError> {
    // Fast path: base account with no extensions
    if account.data_len() == BASE_TOKEN_ACCOUNT_SIZE as usize {
        return Ok(());
    }

    // Borrow account data to get extensions
    let mut account_data = account
        .try_borrow_mut_data()
        .map_err(convert_program_error)?;
    let (ctoken, _) = CToken::zero_copy_at_mut_checked(&mut account_data)?;

    let mut current_slot = 0;
    let top_up_amount =
        process_compressible_extension(ctoken.extensions.as_deref(), account, &mut current_slot)?;

    // Drop borrow before CPI
    drop(account_data);

    if let Some(amount) = top_up_amount {
        if amount > 0 {
            transfer_lamports_via_cpi(amount, payer, account).map_err(convert_program_error)?;
        }
    }

    Ok(())
}
