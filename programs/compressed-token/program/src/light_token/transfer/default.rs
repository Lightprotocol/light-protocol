use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::transfer::process_transfer;

use super::shared::{process_transfer_extensions_transfer, TransferAccounts};

/// Account indices for CToken transfer instruction
const ACCOUNT_SOURCE: usize = 0;
const ACCOUNT_DESTINATION: usize = 1;
const ACCOUNT_AUTHORITY: usize = 2;

/// Process ctoken transfer instruction
///
/// Instruction data format (backwards compatible):
/// - 8 bytes: amount (legacy, no max_top_up enforcement)
/// - 10 bytes: amount + max_top_up (u16, 0 = no limit)
#[profile]
#[inline(always)]
pub fn process_ctoken_transfer(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if accounts.len() < 3 {
        msg!(
            "CToken transfer: expected at least 3 accounts received {}",
            accounts.len()
        );
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Validate minimum instruction data length
    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Hot path: 165-byte accounts have no extensions, skip all extension processing
    let source = accounts
        .get(ACCOUNT_SOURCE)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let destination = accounts
        .get(ACCOUNT_DESTINATION)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    if source.data_len() == 165 && destination.data_len() == 165 {
        return process_transfer(accounts, &instruction_data[..8], false)
            .map_err(|e| ProgramError::Custom(u64::from(e) as u32));
    }

    // Parse max_top_up based on instruction data length
    // 0 means no limit
    let max_top_up = match instruction_data.len() {
        8 => 0u16, // Legacy: no max_top_up
        10 => u16::from_le_bytes(
            instruction_data[8..10]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        ),
        _ => return Err(ProgramError::InvalidInstructionData),
    };

    let signer_is_validated = process_extensions(accounts, max_top_up)?;

    // Only pass the first 8 bytes (amount) to the SPL transfer processor
    process_transfer(accounts, &instruction_data[..8], signer_is_validated)
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))
}

fn process_extensions(
    accounts: &[pinocchio::account_info::AccountInfo],
    max_top_up: u16,
) -> Result<bool, ProgramError> {
    let source = accounts
        .get(ACCOUNT_SOURCE)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let destination = accounts
        .get(ACCOUNT_DESTINATION)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let authority = accounts
        .get(ACCOUNT_AUTHORITY)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    // Ignore decimals - only used for transfer_checked
    let (signer_is_validated, _decimals) = process_transfer_extensions_transfer(
        TransferAccounts {
            source,
            destination,
            authority,
            mint: None,
        },
        max_top_up,
    )?;
    Ok(signer_is_validated)
}
