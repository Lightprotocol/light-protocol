use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::{
    mint_to::process_mint_to, mint_to_checked::process_mint_to_checked,
};

use crate::shared::{
    compressible_top_up::calculate_and_execute_compressible_top_ups, convert_pinocchio_token_error,
};

/// Process ctoken mint_to instruction
///
/// Instruction data format (same as CTokenTransfer):
/// - 8 bytes: amount (legacy, no max_top_up enforcement)
/// - 10 bytes: amount + max_top_up (u16, 0 = no limit)
///
/// Account layout:
/// 0: CMint account (writable)
/// 1: destination CToken account (writable)
/// 2: authority (signer, also payer for top-ups)
#[profile]
#[inline(always)]
pub fn process_ctoken_mint_to(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if accounts.len() < 3 {
        msg!(
            "CToken mint_to: expected at least 3 accounts received {}",
            accounts.len()
        );
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Parse max_top_up (same pattern as ctoken_transfer.rs)
    let max_top_up = match instruction_data.len() {
        8 => 0u16,
        10 => u16::from_le_bytes(
            instruction_data[8..10]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        ),
        _ => return Err(ProgramError::InvalidInstructionData),
    };

    // Call pinocchio mint_to - handles supply/balance updates, authority check, frozen check
    process_mint_to(accounts, &instruction_data[..8]).map_err(convert_pinocchio_token_error)?;

    // Calculate and execute top-ups for both CMint and CToken
    // mint_to account order: [cmint, ctoken, authority]
    // SAFETY: accounts.len() >= 3 validated at function entry
    let cmint = &accounts[0];
    let ctoken = &accounts[1];
    let payer = accounts.get(2);

    calculate_and_execute_compressible_top_ups(cmint, ctoken, payer, max_top_up)
}

/// Process ctoken mint_to_checked instruction
///
/// Instruction data format:
/// - 9 bytes: amount (8) + decimals (1) - legacy, no max_top_up enforcement
/// - 11 bytes: amount (8) + decimals (1) + max_top_up (2, u16, 0 = no limit)
///
/// Account layout (same as mint_to):
/// 0: CMint account (writable)
/// 1: destination CToken account (writable)
/// 2: authority (signer, also payer for top-ups)
#[profile]
#[inline(always)]
pub fn process_ctoken_mint_to_checked(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if accounts.len() < 3 {
        msg!(
            "CToken mint_to_checked: expected at least 3 accounts received {}",
            accounts.len()
        );
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    if instruction_data.len() < 9 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Parse max_top_up from bytes 9-10 if present
    let max_top_up = match instruction_data.len() {
        9 => 0u16, // Legacy: no max_top_up
        11 => u16::from_le_bytes(
            instruction_data[9..11]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        ),
        _ => return Err(ProgramError::InvalidInstructionData),
    };

    // Call pinocchio mint_to_checked - validates decimals against CMint, handles supply/balance updates
    process_mint_to_checked(accounts, &instruction_data[..9])
        .map_err(convert_pinocchio_token_error)?;

    // Calculate and execute top-ups for both CMint and CToken
    // mint_to account order: [cmint, ctoken, authority]
    // SAFETY: accounts.len() >= 3 validated at function entry
    let cmint = &accounts[0];
    let ctoken = &accounts[1];
    let payer = accounts.get(2);

    calculate_and_execute_compressible_top_ups(cmint, ctoken, payer, max_top_up)
}
