use anchor_lang::solana_program::program_error::ProgramError;
use light_ctoken_interface::{state::CToken, CTokenError};
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::{approve::process_approve, revoke::process_revoke};

use crate::shared::{
    compressible_top_up::process_compression_top_up, convert_pinocchio_token_error,
    convert_program_error, transfer_lamports_via_cpi,
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
    process_approve(accounts, &instruction_data[..8]).map_err(convert_pinocchio_token_error)?;
    // Hot path: 165-byte accounts have no extensions, just call pinocchio directly
    if source.data_len() == 165 {
        return Ok(());
    }

    let payer = accounts.get(APPROVE_ACCOUNT_OWNER);

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
    process_compressible_top_up(source, payer, max_top_up)
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

    process_revoke(accounts).map_err(convert_pinocchio_token_error)?;

    // Hot path: 165-byte accounts have no extensions
    if source.data_len() == 165 {
        return Ok(());
    }

    let payer = accounts.get(REVOKE_ACCOUNT_OWNER);

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

    process_compressible_top_up(source, payer, max_top_up)
}

/// Calculate and transfer compressible top-up for a single account.
///
/// # Arguments
/// * `max_top_up` - Maximum lamports for top-up. Transaction fails if exceeded. (0 = no limit)
#[inline(always)]
fn process_compressible_top_up(
    account: &AccountInfo,
    payer: Option<&AccountInfo>,
    max_top_up: u16,
) -> Result<(), ProgramError> {
    let ctoken = CToken::from_account_info_mut_checked(account)?;

    // Only process top-up if account has Compressible extension
    let transfer_amount = if let Some(compressible) = ctoken.get_compressible_extension() {
        let mut transfer_amount = 0u64;

        process_compression_top_up(
            &compressible.info,
            account,
            &mut 0,
            &mut transfer_amount,
            &mut 0,
            &mut None,
        )?;

        if max_top_up > 0 && (max_top_up as u64) < transfer_amount {
            return Err(CTokenError::MaxTopUpExceeded.into());
        }
        transfer_amount
    } else {
        0
    };

    if transfer_amount > 0 {
        let payer = payer.ok_or(CTokenError::MissingPayer)?;
        transfer_lamports_via_cpi(transfer_amount, payer, account)
            .map_err(convert_program_error)?;
    }

    Ok(())
}
