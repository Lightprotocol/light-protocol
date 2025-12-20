use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::transfer_checked::process_transfer_checked;

use super::shared::{process_transfer_extensions, TransferAccounts};
use crate::shared::owner_validation::check_token_program_owner;
/// Account indices for CToken transfer_checked instruction
/// Note: Different from ctoken_transfer - mint is at index 1
const ACCOUNT_SOURCE: usize = 0;
const ACCOUNT_MINT: usize = 1;
const ACCOUNT_DESTINATION: usize = 2;
const ACCOUNT_AUTHORITY: usize = 3;

/// Process ctoken transfer_checked instruction
///
/// Instruction data format (backwards compatible):
/// - 9 bytes: amount + decimals (legacy, no max_top_up enforcement)
/// - 11 bytes: amount + decimals + max_top_up (u16, 0 = no limit)
#[profile]
#[inline(always)]
pub fn process_ctoken_transfer_checked(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if accounts.len() < 4 {
        msg!(
            "CToken transfer_checked: expected at least 4 accounts received {}",
            accounts.len()
        );
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Validate minimum instruction data length (amount + decimals)
    if instruction_data.len() < 9 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Get account references
    let source = accounts
        .get(ACCOUNT_SOURCE)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let mint = accounts
        .get(ACCOUNT_MINT)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let destination = accounts
        .get(ACCOUNT_DESTINATION)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let authority = accounts
        .get(ACCOUNT_AUTHORITY)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    // Validate mint ownership before any other processing
    check_token_program_owner(mint)?;

    // Parse max_top_up based on instruction data length
    // 0 means no limit
    let max_top_up = match instruction_data.len() {
        9 => 0u16, // Legacy: no max_top_up
        11 => u16::from_le_bytes(
            instruction_data[9..11]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        ),
        _ => return Err(ProgramError::InvalidInstructionData),
    };

    let signer_is_validated = process_transfer_extensions(
        TransferAccounts {
            source,
            destination,
            authority,
            mint: Some(mint),
        },
        max_top_up,
    )?;

    // Pass the first 9 bytes (amount + decimals) to the SPL transfer_checked processor
    process_transfer_checked(accounts, &instruction_data[..9], signer_is_validated)
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))
}
