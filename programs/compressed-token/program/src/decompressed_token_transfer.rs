use account_compression::utils::transfer_lamports::transfer_lamports_cpi;
use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_ctoken_types::{
    state::{CompressedToken, ZExtensionStruct},
    CTokenError,
};
use light_profiler::profile;
use light_zero_copy::traits::ZeroCopyAt;
use pinocchio::account_info::AccountInfo;
use spl_token::instruction::TokenInstruction;

use crate::{convert_account_infos::convert_account_infos, MAX_ACCOUNTS};

/// Process decompressed token transfer instruction
#[profile]
pub fn process_decompressed_token_transfer(
    accounts: &[AccountInfo],
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
            let transfer_amounts =
                update_compressible_accounts_last_written_slot(account_infos.as_slice())?;
            for (amount, account) in transfer_amounts.iter().zip(account_infos.iter().take(2)) {
                if *amount > 0 {
                    let payer = account_infos
                        .get(2)
                        .ok_or(ProgramError::NotEnoughAccountKeys)?;
                    transfer_lamports_cpi(payer, account, *amount)?;
                }
            }
        }
        _ => return Err(ProgramError::InvalidInstructionData),
    }
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
/// Update last_written_slot for token accounts with compressible extensions
/// SPL token transfer uses accounts[0] as source and accounts[1] as destination
#[inline(always)]
#[profile]
fn update_compressible_accounts_last_written_slot(
    accounts: &[anchor_lang::prelude::AccountInfo],
) -> Result<[u64; 2], ProgramError> {
    let mut transfers = [0u64; 2];
    // Update sender (accounts[0]) and recipient (accounts[1])
    // if these have extensions.
    for (i, account) in accounts.iter().take(2).enumerate() {
        if account.data_len() > light_ctoken_types::BASE_TOKEN_ACCOUNT_SIZE as usize {
            let account_data = account.try_borrow_data()?;
            let (token, _) = CompressedToken::zero_copy_at(&account_data)?;
            if let Some(extensions) = token.extensions.as_ref() {
                for extension in extensions.iter() {
                    if let ZExtensionStruct::Compressible(compressible_extension) = extension {
                        {
                            if let Some(write_top_up_lamports) =
                                compressible_extension.write_top_up_lamports.as_ref()
                            {
                                transfers[i] = write_top_up_lamports.get() as u64;
                            }

                            use pinocchio::sysvars::{clock::Clock, Sysvar};
                            let current_slot = Clock::get()
                                .map_err(|_| CTokenError::SysvarAccessError)?
                                .slot;

                            let (is_compressible, required_funds) = compressible_extension
                                .is_compressible(
                                    account.data_len() as u64,
                                    current_slot,
                                    account.lamports(),
                                );
                            if is_compressible {
                                transfers[i] += required_funds;
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(transfers)
}
