use anchor_lang::solana_program::program_error::ProgramError;
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::{approve::process_approve, revoke::process_revoke};
#[cfg(target_os = "solana")]
use {
    crate::shared::{convert_program_error, transfer_lamports_via_cpi},
    light_token_interface::state::top_up_lamports_from_account_info_unchecked,
    light_token_interface::TokenError,
};

use crate::shared::convert_pinocchio_token_error;

/// Approve: 8-byte base (amount), owner at index 2, fee_payer at index 4 (optional)
const APPROVE_BASE_LEN: usize = 8;
const APPROVE_OWNER_IDX: usize = 2;
// System program is index 3
const APPROVE_FEE_PAYER_IDX: usize = 4;

/// Revoke: 0-byte base, owner at index 1, fee_payer at index 3 (optional)
const REVOKE_BASE_LEN: usize = 0;
const REVOKE_OWNER_IDX: usize = 1;
// System program is index 2
const REVOKE_FEE_PAYER_IDX: usize = 3;

/// Process CToken approve instruction.
/// Handles compressible extension top-up after delegating to pinocchio.
///
/// Instruction data format (backwards compatible):
/// - 8 bytes: amount (legacy, no max_top_up enforcement)
/// - 10 bytes: amount + max_top_up (u16, u16::MAX = no limit, 0 = no top-ups allowed)
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
    handle_compressible_top_up::<APPROVE_BASE_LEN, APPROVE_OWNER_IDX, APPROVE_FEE_PAYER_IDX>(
        accounts,
        instruction_data,
    )
}

/// Process CToken revoke instruction.
/// Handles compressible extension top-up after delegating to pinocchio.
///
/// Instruction data format (backwards compatible):
/// - 0 bytes: legacy, no max_top_up enforcement
/// - 2 bytes: max_top_up (u16, u16::MAX = no limit, 0 = no top-ups allowed)
#[inline(always)]
pub fn process_ctoken_revoke(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if accounts.is_empty() {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    process_revoke(accounts).map_err(convert_pinocchio_token_error)?;
    handle_compressible_top_up::<REVOKE_BASE_LEN, REVOKE_OWNER_IDX, REVOKE_FEE_PAYER_IDX>(
        accounts,
        instruction_data,
    )
}

/// Handle compressible extension top-up after pinocchio processing.
///
/// # Type Parameters
/// * `BASE_LEN` - Base instruction data length (8 for approve, 0 for revoke)
/// * `OWNER_IDX` - Index of owner account (2 for approve, 1 for revoke)
/// * `FEE_PAYER_IDX` - Index of optional fee payer account (4 for approve, 3 for revoke)
#[inline(always)]
fn handle_compressible_top_up<
    const BASE_LEN: usize,
    const OWNER_IDX: usize,
    const FEE_PAYER_IDX: usize,
>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let source = &accounts[0];

    // Hot path: 165-byte accounts have no extensions
    if source.data_len() == 165 {
        return Ok(());
    }

    process_compressible_top_up::<BASE_LEN, OWNER_IDX, FEE_PAYER_IDX>(
        source,
        accounts,
        instruction_data,
    )
}

/// Calculate and transfer compressible top-up for a single ctoken account.
///
/// # Type Parameters
/// * `BASE_LEN` - Base instruction data length (8 for approve, 0 for revoke)
/// * `OWNER_IDX` - Index of owner account (2 for approve, 1 for revoke)
/// * `FEE_PAYER_IDX` - Index of optional fee payer account (4 for approve, 3 for revoke)
#[cold]
#[allow(unused)]
fn process_compressible_top_up<
    const BASE_LEN: usize,
    const OWNER_IDX: usize,
    const FEE_PAYER_IDX: usize,
>(
    account: &AccountInfo,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Returns None if no Compressible extension, Some(amount) otherwise
    #[cfg(target_os = "solana")]
    {
        let authority_payer = accounts.get(OWNER_IDX);
        let fee_payer = accounts.get(FEE_PAYER_IDX);
        let payer = fee_payer.or(authority_payer);

        // u16::MAX means no limit, 0 means no top-ups allowed
        let max_top_up = match instruction_data.len() {
            len if len == BASE_LEN => u16::MAX, // Legacy: no max_top_up limit
            len if len == BASE_LEN + 2 => u16::from_le_bytes(
                instruction_data[BASE_LEN..BASE_LEN + 2]
                    .try_into()
                    .map_err(|_| ProgramError::InvalidInstructionData)?,
            ),
            _ => return Err(ProgramError::InvalidInstructionData),
        };

        let transfer_amount = {
            let mut current_slot = 0;
            top_up_lamports_from_account_info_unchecked(account, &mut current_slot).unwrap_or(0)
        };

        if transfer_amount > 0 {
            // u16::MAX means no limit, 0 means no top-ups allowed
            // max_top_up is in units of 1,000 lamports (max ~65.5M lamports).
            if max_top_up != u16::MAX && transfer_amount > (max_top_up as u64).saturating_mul(1000)
            {
                return Err(TokenError::MaxTopUpExceeded.into());
            }
            let payer = payer.ok_or(TokenError::MissingPayer)?;
            transfer_lamports_via_cpi(transfer_amount, payer, account)
                .map_err(convert_program_error)?;
        }
    }

    Ok(())
}
