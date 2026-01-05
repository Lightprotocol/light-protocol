use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::{burn::process_burn, burn_checked::process_burn_checked};

use crate::shared::compressible_top_up::calculate_and_execute_compressible_top_ups;

/// Process ctoken burn instruction
///
/// Instruction data format (same as CTokenTransfer/CTokenMintTo):
/// - 8 bytes: amount (legacy, no max_top_up enforcement)
/// - 10 bytes: amount + max_top_up (u16, 0 = no limit)
///
/// Account layout:
/// 0: source CToken account (writable)
/// 1: CMint account (writable)
/// 2: authority (signer, also payer for top-ups)
#[profile]
#[inline(always)]
pub fn process_ctoken_burn(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if accounts.len() < 3 {
        msg!(
            "CToken burn: expected at least 3 accounts received {}",
            accounts.len()
        );
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Parse max_top_up
    let max_top_up = match instruction_data.len() {
        8 => 0u16,
        10 => u16::from_le_bytes(
            instruction_data[8..10]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        ),
        _ => return Err(ProgramError::InvalidInstructionData),
    };

    // Call pinocchio burn - handles balance/supply updates, authority check, frozen check
    process_burn(accounts, &instruction_data[..8])
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;

    // Calculate and execute top-ups for both CMint and CToken
    // burn account order: [ctoken, cmint, authority] - reverse of mint_to
    let ctoken = accounts.first().ok_or(ProgramError::NotEnoughAccountKeys)?;
    let cmint = accounts.get(1).ok_or(ProgramError::NotEnoughAccountKeys)?;
    let payer = accounts.get(2).ok_or(ProgramError::NotEnoughAccountKeys)?;

    calculate_and_execute_compressible_top_ups(cmint, ctoken, payer, max_top_up)
}

/// Process ctoken burn_checked instruction
///
/// Instruction data format:
/// - 9 bytes: amount (8) + decimals (1) - legacy, no max_top_up enforcement
/// - 11 bytes: amount (8) + decimals (1) + max_top_up (2, u16, 0 = no limit)
///
/// Account layout (same as burn):
/// 0: source CToken account (writable)
/// 1: CMint account (writable)
/// 2: authority (signer, also payer for top-ups)
#[profile]
#[inline(always)]
pub fn process_ctoken_burn_checked(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if accounts.len() < 3 {
        msg!(
            "CToken burn_checked: expected at least 3 accounts received {}",
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

    // Call pinocchio burn_checked - validates decimals against CMint, handles balance/supply updates
    process_burn_checked(accounts, &instruction_data[..9])
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;

    // Calculate and execute top-ups for both CMint and CToken
    // burn account order: [ctoken, cmint, authority] - reverse of mint_to
    let ctoken = accounts.first().ok_or(ProgramError::NotEnoughAccountKeys)?;
    let cmint = accounts.get(1).ok_or(ProgramError::NotEnoughAccountKeys)?;
    let payer = accounts.get(2).ok_or(ProgramError::NotEnoughAccountKeys)?;

    calculate_and_execute_compressible_top_ups(cmint, ctoken, payer, max_top_up)
}
