use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::{
    shared::transfer::process_transfer, transfer_checked::process_transfer_checked,
    unpack_amount_and_decimals,
};

use super::shared::{process_transfer_extensions_transfer_checked, TransferAccounts};
use crate::shared::{convert_pinocchio_token_error, owner_validation::check_token_program_owner};
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

    // SAFETY: accounts.len() >= 4 validated at function entry
    let source = &accounts[ACCOUNT_SOURCE];
    let destination = &accounts[ACCOUNT_DESTINATION];

    // Hot path: 165-byte accounts have no extensions, skip all extension processing
    if source.data_len() == 165 && destination.data_len() == 165 {
        return process_transfer_checked(accounts, &instruction_data[..9], false)
            .map_err(convert_pinocchio_token_error);
    }

    let mint = &accounts[ACCOUNT_MINT];
    let authority = &accounts[ACCOUNT_AUTHORITY];

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

    let (signer_is_validated, extension_decimals) = process_transfer_extensions_transfer_checked(
        TransferAccounts {
            source,
            destination,
            authority,
            mint: Some(mint),
        },
        max_top_up,
    )?;

    // Pass the first 9 bytes (amount + decimals) to the SPL transfer_checked processor
    let (amount, decimals) =
        unpack_amount_and_decimals(instruction_data).map_err(|e| ProgramError::Custom(e as u32))?;

    if let Some(extension_decimals) = extension_decimals {
        if extension_decimals != decimals {
            msg!("extension_decimals != decimals");
            return Err(ProgramError::InvalidInstructionData);
        }
        // Create accounts slice without mint: [source, destination, authority]
        // pinocchio expects 3 accounts when expected_decimals is None
        let transfer_accounts = [*source, *destination, *authority];
        process_transfer(
            transfer_accounts.as_slice(),
            amount,
            None,
            signer_is_validated,
        )
        .map_err(convert_pinocchio_token_error)
    } else {
        check_token_program_owner(mint)?;
        process_transfer(accounts, amount, Some(decimals), signer_is_validated)
            .map_err(convert_pinocchio_token_error)
    }
}
