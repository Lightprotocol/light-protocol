use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_ctoken_types::{
    state::{CToken, ZExtensionStruct},
    CTokenError,
};
use light_profiler::profile;
use light_zero_copy::traits::ZeroCopyAt;
use pinocchio::account_info::AccountInfo;
use spl_token::instruction::TokenInstruction;

use crate::{
    convert_account_infos::convert_account_infos,
    shared::transfer_lamports::{multi_transfer_lamports, Transfer},
    MAX_ACCOUNTS,
};

/// Process decompressed token transfer instruction
#[profile]
pub fn process_decompressed_token_transfer<'a>(
    accounts: &'a [AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if accounts.len() < 3 {
        msg!(
            "Decompressed transfer: expected at least 3 accounts received {}",
            accounts.len()
        );
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let instruction = TokenInstruction::unpack(&instruction_data[1..])?;
    match instruction {
        TokenInstruction::Transfer { amount } => {
            let account_infos = unsafe { convert_account_infos::<MAX_ACCOUNTS>(accounts)? };
            process_light_token_transfer(&crate::ID, &account_infos, amount)?;
        }
        _ => return Err(ProgramError::InvalidInstructionData),
    };
    calculate_and_execute_top_up_transfers(accounts)?;
    Ok(())
}

// Note:
//  We need to use light_token_22 fork for token_22 contains
//  a hardcoded program id check for account ownership.
#[profile]
fn process_light_token_transfer(
    program_id: &anchor_lang::prelude::Pubkey,
    account_infos: &[anchor_lang::prelude::AccountInfo],
    amount: u64,
) -> Result<(), ProgramError> {
    light_token_22::processor::Processor::process_transfer(
        program_id,
        account_infos,
        amount,
        None,
        None,
    )
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
                .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;
            let (token, _) = CToken::zero_copy_at(&account_data)?;
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

    let payer = accounts.get(2).ok_or(ProgramError::NotEnoughAccountKeys)?;
    multi_transfer_lamports(payer, &transfers)
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;

    Ok(())
}
