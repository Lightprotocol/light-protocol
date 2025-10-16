use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_ctoken_types::{
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
#[profile]
#[inline(always)]
pub fn process_ctoken_transfer<'a>(
    accounts: &'a [AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if accounts.len() < 3 {
        msg!(
            "CToken transfer: expected at least 3 accounts received {}",
            accounts.len()
        );
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    process_transfer(accounts, instruction_data)
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;
    calculate_and_execute_top_up_transfers(accounts)
}

/// Calculate and execute top-up transfers for compressible accounts
#[inline(always)]
#[profile]
fn calculate_and_execute_top_up_transfers(
    accounts: &[pinocchio::account_info::AccountInfo],
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
    // Calculate transfer amounts for accounts with compressible extensions
    for transfer in transfers.iter_mut() {
        if transfer.account.data_len() > light_ctoken_types::BASE_TOKEN_ACCOUNT_SIZE as usize {
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
                                compressible_extension.lamports_per_write.into(),
                                light_ctoken_types::COMPRESSIBLE_TOKEN_RENT_EXEMPTION,
                            )
                            .map_err(|_| CTokenError::InvalidAccountData)?;
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
    } else {
        let payer = accounts.get(2).ok_or(ProgramError::NotEnoughAccountKeys)?;
        multi_transfer_lamports(payer, &transfers).map_err(convert_program_error)?;
    }
    Ok(())
}
