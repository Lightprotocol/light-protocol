use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_ctoken_interface::{state::CToken, CTokenError};
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::{
    approve::process_approve, revoke::process_revoke,
    shared::approve::process_approve as shared_process_approve, unpack_amount_and_decimals,
};

use crate::{
    shared::{
        convert_program_error, owner_validation::check_token_program_owner,
        transfer_lamports_via_cpi,
    },
    transfer2::compression::ctoken::process_compression_top_up,
};

/// Account indices for approve instruction
const APPROVE_ACCOUNT_SOURCE: usize = 0;
const APPROVE_ACCOUNT_OWNER: usize = 2; // owner is payer for top-up

/// Account indices for approve_checked instruction (static 4-account layout)
const APPROVE_CHECKED_ACCOUNT_SOURCE: usize = 0;
const APPROVE_CHECKED_ACCOUNT_MINT: usize = 1;
const APPROVE_CHECKED_ACCOUNT_DELEGATE: usize = 2;
const APPROVE_CHECKED_ACCOUNT_OWNER: usize = 3;

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
    // Borrow account data to get extensions
    let mut account_data = account
        .try_borrow_mut_data()
        .map_err(convert_program_error)?;
    let (ctoken, _) = CToken::zero_copy_at_mut_checked(&mut account_data)?;

    let mut transfer_amount = 0u64;
    let mut lamports_budget = if max_top_up == 0 {
        u64::MAX
    } else {
        (max_top_up as u64).saturating_add(1)
    };

    process_compression_top_up(
        &ctoken.base.compression,
        account,
        &mut 0,
        &mut transfer_amount,
        &mut lamports_budget,
    )?;

    // Drop borrow before CPI
    drop(account_data);

    if transfer_amount > 0 {
        if lamports_budget == 0 {
            return Err(CTokenError::MaxTopUpExceeded.into());
        }
        transfer_lamports_via_cpi(transfer_amount, payer, account)
            .map_err(convert_program_error)?;
    }

    Ok(())
}

/// Process CToken approve_checked instruction.
/// Static 4-account layout with cached decimals optimization.
///
/// Instruction data format:
/// - 9 bytes: amount (8) + decimals (1) - legacy, no max_top_up enforcement
/// - 11 bytes: amount (8) + decimals (1) + max_top_up (2, u16, 0 = no limit)
///
/// Account layout (always 4 accounts):
/// 0: source CToken account (writable) - may have cached decimals
/// 1: mint account (immutable) - used for validation if no cached decimals
/// 2: delegate (immutable) - the delegate authority
/// 3: owner (signer, writable) - owner of source, payer for top-ups
#[inline(always)]
pub fn process_ctoken_approve_checked(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        msg!(
            "CToken approve_checked: expected at least 4 accounts received {}",
            accounts.len()
        );
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    if instruction_data.len() < 9 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Parse amount and decimals from instruction data
    let (amount, decimals) =
        unpack_amount_and_decimals(instruction_data).map_err(|e| ProgramError::Custom(e as u32))?;

    // Parse max_top_up from bytes 9-10 if present (0 = no limit)
    let max_top_up = match instruction_data.len() {
        9 => 0u16, // Legacy: no max_top_up
        11 => u16::from_le_bytes(
            instruction_data[9..11]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        ),
        _ => return Err(ProgramError::InvalidInstructionData),
    };

    let source = accounts
        .get(APPROVE_CHECKED_ACCOUNT_SOURCE)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let mint = accounts
        .get(APPROVE_CHECKED_ACCOUNT_MINT)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let delegate = accounts
        .get(APPROVE_CHECKED_ACCOUNT_DELEGATE)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let owner = accounts
        .get(APPROVE_CHECKED_ACCOUNT_OWNER)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    // Borrow source account to check for cached decimals
    let cached_decimals = {
        let mut account_data = source
            .try_borrow_mut_data()
            .map_err(convert_program_error)?;
        let (ctoken, _) = CToken::zero_copy_at_mut_checked(&mut account_data)?;

        // Get cached decimals if present
        let cached = ctoken.base.decimals();

        // Also handle compressible top-up while we have the borrow
        let mut transfer_amount = 0u64;
        let mut lamports_budget = if max_top_up == 0 {
            u64::MAX
        } else {
            (max_top_up as u64).saturating_add(1)
        };

        process_compression_top_up(
            &ctoken.base.compression,
            source,
            &mut 0,
            &mut transfer_amount,
            &mut lamports_budget,
        )?;

        // Drop borrow before CPI
        drop(account_data);

        if transfer_amount > 0 {
            if lamports_budget == 0 {
                return Err(CTokenError::MaxTopUpExceeded.into());
            }
            transfer_lamports_via_cpi(transfer_amount, owner, source)
                .map_err(convert_program_error)?;
        }

        cached
    };

    // Call pinocchio approve based on cached decimals presence
    if let Some(cached_decimals) = cached_decimals {
        // Validate cached decimals match instruction decimals
        if cached_decimals != decimals {
            msg!(
                "CToken approve_checked: cached decimals {} != instruction decimals {}",
                cached_decimals,
                decimals
            );
            return Err(ProgramError::InvalidInstructionData);
        }
        // Create 3-account slice [source, delegate, owner] - skip mint
        let approve_accounts = [*source, *delegate, *owner];
        shared_process_approve(&approve_accounts, amount, None).map_err(convert_program_error)
    } else {
        // No cached decimals - validate via mint account
        check_token_program_owner(mint)?;
        // Use full 4-account layout [source, mint, delegate, owner]
        shared_process_approve(accounts, amount, Some(decimals)).map_err(convert_program_error)
    }
}
