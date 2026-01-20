use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::transfer::process_transfer;

use super::shared::{process_transfer_extensions_transfer, TransferAccounts};
use crate::shared::convert_pinocchio_token_error;

/// Account indices for CToken transfer instruction
const ACCOUNT_SOURCE: usize = 0;
const ACCOUNT_DESTINATION: usize = 1;
const ACCOUNT_AUTHORITY: usize = 2;
#[allow(dead_code)]
const ACCOUNT_SYSTEM_PROGRAM: usize = 3;
const ACCOUNT_FEE_PAYER: usize = 4;

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
    // SAFETY: accounts.len() >= 3 validated at function entry
    let source = &accounts[ACCOUNT_SOURCE];
    let destination = &accounts[ACCOUNT_DESTINATION];
    if source.data_len() == 165 && destination.data_len() == 165 {
        return process_transfer(accounts, &instruction_data[..8], false)
            .map_err(convert_pinocchio_token_error);
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
        .map_err(convert_pinocchio_token_error)
}

fn process_extensions(accounts: &[AccountInfo], max_top_up: u16) -> Result<bool, ProgramError> {
    // SAFETY: accounts.len() >= 3 validated in caller
    let source = &accounts[ACCOUNT_SOURCE];
    let destination = &accounts[ACCOUNT_DESTINATION];
    let authority = &accounts[ACCOUNT_AUTHORITY];
    let fee_payer = accounts.get(ACCOUNT_FEE_PAYER);

    let (signer_is_validated, _) = process_transfer_extensions_transfer(
        TransferAccounts {
            source,
            destination,
            authority,
            mint: None,
            fee_payer,
        },
        max_top_up,
    )?;
    Ok(signer_is_validated)
}
