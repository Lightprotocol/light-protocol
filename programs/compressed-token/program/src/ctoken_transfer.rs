use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_ctoken_interface::{
    state::{CToken, ZExtensionStruct},
    CTokenError,
};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::transfer::process_transfer;

use crate::shared::{
    convert_program_error,
    transfer_lamports::{multi_transfer_lamports, Transfer},
};

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

    // Only pass the first 8 bytes (amount) to the SPL transfer processor
    process_transfer(accounts, &instruction_data[..8])
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;
    calculate_and_execute_top_up_transfers(accounts, max_top_up)
}

/// Calculate and execute top-up transfers for compressible accounts
///
/// # Arguments
/// * `accounts` - The account infos (source, dest, authority/payer)
/// * `max_top_up` - Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
#[inline(always)]
#[profile]
fn calculate_and_execute_top_up_transfers(
    accounts: &[pinocchio::account_info::AccountInfo],
    max_top_up: u16,
) -> Result<(), ProgramError> {
    // Initialize transfers array with account references, amounts will be updated
    let account0 = accounts.first().ok_or(ProgramError::NotEnoughAccountKeys)?;
    let account1 = accounts.get(1).ok_or(ProgramError::NotEnoughAccountKeys)?;
    let mut transfers = [
        Transfer {
            account: account0,
            amount: 0,
        },
        Transfer {
            account: account1,
            amount: 0,
        },
    ];
    let mut current_slot = 0;
    // Initialize budget: +1 allows exact match (total == max_top_up)
    let mut lamports_budget = (max_top_up as u64).saturating_add(1);

    // Calculate transfer amounts for accounts with compressible extensions
    for transfer in transfers.iter_mut() {
        if transfer.account.data_len() > light_ctoken_interface::BASE_TOKEN_ACCOUNT_SIZE as usize {
            let account_data = transfer
                .account
                .try_borrow_data()
                .map_err(convert_program_error)?;
            let (token, _) = CToken::zero_copy_at_checked(&account_data)?;
            if let Some(extensions) = token.extensions.as_ref() {
                for extension in extensions.iter() {
                    if let ZExtensionStruct::Compressible(compressible_extension) = extension {
                        if current_slot == 0 {
                            use pinocchio::sysvars::{clock::Clock, Sysvar};
                            current_slot = Clock::get()
                                .map_err(|_| CTokenError::SysvarAccessError)?
                                .slot;
                        }

                        transfer.amount = compressible_extension
                            .calculate_top_up_lamports(
                                transfer.account.data_len() as u64,
                                current_slot,
                                transfer.account.lamports(),
                                light_ctoken_interface::COMPRESSIBLE_TOKEN_RENT_EXEMPTION,
                            )
                            .map_err(|_| CTokenError::InvalidAccountData)?;

                        lamports_budget = lamports_budget.saturating_sub(transfer.amount);
                    }
                }
            } else {
                // Only Compressible extensions are implemented for ctoken accounts.
                return Err(CTokenError::InvalidAccountData.into());
            }
        }
    }
    // Exit early in case none of the accounts is compressible.
    if current_slot == 0 {
        return Ok(());
    }

    if transfers[0].amount == 0 && transfers[1].amount == 0 {
        return Ok(());
    }

    // Check budget wasn't exhausted (0 means exceeded max_top_up)
    if max_top_up != 0 && lamports_budget == 0 {
        return Err(CTokenError::MaxTopUpExceeded.into());
    }

    let payer = accounts.get(2).ok_or(ProgramError::NotEnoughAccountKeys)?;
    multi_transfer_lamports(payer, &transfers).map_err(convert_program_error)?;
    Ok(())
}
