use anchor_lang::solana_program::program_error::ProgramError;
use light_ctoken_interface::{state::CToken, CTokenError, BASE_TOKEN_ACCOUNT_SIZE};
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::{approve::process_approve, revoke::process_revoke};

use crate::{
    shared::{convert_program_error, transfer_lamports_via_cpi},
    transfer2::compression::ctoken::process_compression_top_up,
};

/// Account indices for approve instruction
const APPROVE_ACCOUNT_SOURCE: usize = 0;
const APPROVE_ACCOUNT_OWNER: usize = 2; // owner is payer for top-up

/// Account indices for revoke instruction
const REVOKE_ACCOUNT_SOURCE: usize = 0;
const REVOKE_ACCOUNT_OWNER: usize = 1; // owner is payer for top-up

/// Process CToken approve instruction.
/// Handles compressible extension top-up before delegating to pinocchio.
///
/// Instruction data format (backwards compatible):
/// - 8 bytes: amount (legacy, no max_top_up enforcement)
/// - 10 bytes: amount + max_top_up (u16, 0 = no limit)
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

    // Parse max_top_up based on instruction data length (0 = no limit)
    let max_top_up = match instruction_data.len() {
        8 => 0u16, // Legacy: no max_top_up
        10 => u16::from_le_bytes(
            instruction_data[8..10]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        ),
        _ => return Err(ProgramError::InvalidInstructionData),
    };

    // Handle compressible top-up before pinocchio call
    process_compressible_top_up(source, payer, max_top_up)?;

    // Only pass the first 8 bytes (amount) to the SPL approve processor
    process_approve(accounts, &instruction_data[..8]).map_err(convert_program_error)
}

/// Process CToken revoke instruction.
/// Handles compressible extension top-up before delegating to pinocchio.
///
/// Instruction data format (backwards compatible):
/// - 0 bytes: legacy, no max_top_up enforcement
/// - 2 bytes: max_top_up (u16, 0 = no limit)
#[inline(always)]
pub fn process_ctoken_revoke(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let source = accounts
        .get(REVOKE_ACCOUNT_SOURCE)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let payer = accounts
        .get(REVOKE_ACCOUNT_OWNER)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    // Parse max_top_up based on instruction data length (0 = no limit)
    let max_top_up = match instruction_data.len() {
        0 => 0u16, // Legacy: no max_top_up
        2 => u16::from_le_bytes(
            instruction_data[0..2]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        ),
        _ => return Err(ProgramError::InvalidInstructionData),
    };

    // Handle compressible top-up before pinocchio call
    process_compressible_top_up(source, payer, max_top_up)?;

    process_revoke(accounts).map_err(convert_program_error)
}

/// Calculate and transfer compressible top-up for a single account.
///
/// # Arguments
/// * `max_top_up` - Maximum lamports for top-up. Transaction fails if exceeded. (0 = no limit)
#[inline(always)]
fn process_compressible_top_up(
    account: &AccountInfo,
    payer: &AccountInfo,
    max_top_up: u16,
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
    let mut transfer_amount = 0u64;
    let mut lamports_budget = if max_top_up == 0 {
        u64::MAX
    } else {
        (max_top_up as u64).saturating_add(1)
    };

    process_compression_top_up(
        &ctoken.meta.compression,
        account,
        &mut current_slot,
        &mut transfer_amount,
        &mut lamports_budget,
    )?;

    // Drop borrow before CPI
    drop(account_data);

    if transfer_amount > 0 {
        // Check budget if max_top_up is set (non-zero)
        if max_top_up != 0 && transfer_amount > max_top_up as u64 {
            return Err(CTokenError::MaxTopUpExceeded.into());
        }
        transfer_lamports_via_cpi(transfer_amount, payer, account)
            .map_err(convert_program_error)?;
    }

    Ok(())
}
