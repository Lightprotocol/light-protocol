use anchor_lang::solana_program::program_error::ProgramError;
use light_ctoken_interface::{state::CToken, CTokenError};
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::{approve::process_approve, revoke::process_revoke};

use crate::shared::{
    compressible_top_up::process_compression_top_up, convert_pinocchio_token_error,
    convert_program_error, transfer_lamports_via_cpi,
};

/// Approve: 8-byte base (amount), payer at index 2
const APPROVE_BASE_LEN: usize = 8;
const APPROVE_PAYER_IDX: usize = 2;

/// Revoke: 0-byte base, payer at index 1
const REVOKE_BASE_LEN: usize = 0;
const REVOKE_PAYER_IDX: usize = 1;

/// Process CToken approve instruction.
/// Handles compressible extension top-up after delegating to pinocchio.
///
/// Instruction data format (backwards compatible):
/// - 8 bytes: amount (legacy, no max_top_up enforcement)
/// - 10 bytes: amount + max_top_up (u16, 0 = no limit)
#[inline(always)]
pub fn process_ctoken_approve(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if accounts.is_empty() {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    if instruction_data.len() < APPROVE_BASE_LEN {
        return Err(ProgramError::InvalidInstructionData);
    }
    process_approve(accounts, &instruction_data[..APPROVE_BASE_LEN])
        .map_err(convert_pinocchio_token_error)?;
    handle_compressible_top_up::<APPROVE_BASE_LEN, APPROVE_PAYER_IDX>(accounts, instruction_data)
}

/// Process CToken revoke instruction.
/// Handles compressible extension top-up after delegating to pinocchio.
///
/// Instruction data format (backwards compatible):
/// - 0 bytes: legacy, no max_top_up enforcement
/// - 2 bytes: max_top_up (u16, 0 = no limit)
#[inline(always)]
pub fn process_ctoken_revoke(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if accounts.is_empty() {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    process_revoke(accounts).map_err(convert_pinocchio_token_error)?;
    handle_compressible_top_up::<REVOKE_BASE_LEN, REVOKE_PAYER_IDX>(accounts, instruction_data)
}

/// Handle compressible extension top-up after pinocchio processing.
///
/// # Type Parameters
/// * `BASE_LEN` - Base instruction data length (8 for approve, 0 for revoke)
/// * `PAYER_IDX` - Index of payer account (2 for approve, 1 for revoke)
#[inline(always)]
fn handle_compressible_top_up<const BASE_LEN: usize, const PAYER_IDX: usize>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let source = &accounts[0];

    // Hot path: 165-byte accounts have no extensions
    if source.data_len() == 165 {
        return Ok(());
    }

    process_compressible_top_up::<BASE_LEN, PAYER_IDX>(source, accounts, instruction_data)
}

/// Calculate and transfer compressible top-up for a single ctoken account.
///
/// # Type Parameters
/// * `BASE_LEN` - Base instruction data length (8 for approve, 0 for revoke)
/// * `PAYER_IDX` - Index of payer account (2 for approve, 1 for revoke)
#[cold]
fn process_compressible_top_up<const BASE_LEN: usize, const PAYER_IDX: usize>(
    account: &AccountInfo,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let payer = accounts.get(PAYER_IDX);

    let max_top_up = match instruction_data.len() {
        len if len == BASE_LEN => 0u16,
        len if len == BASE_LEN + 2 => u16::from_le_bytes(
            instruction_data[BASE_LEN..BASE_LEN + 2]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        ),
        _ => return Err(ProgramError::InvalidInstructionData),
    };

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
