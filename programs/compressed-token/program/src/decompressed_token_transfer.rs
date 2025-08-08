use crate::{convert_account_infos::convert_account_infos, MAX_ACCOUNTS};
use anchor_lang::solana_program::program_error::ProgramError;
use light_ctoken_types::state::CompressedToken;
use light_zero_copy::borsh_mut::DeserializeMut;
use pinocchio::account_info::AccountInfo;
use spl_token::instruction::TokenInstruction;

/// Process decompressed token transfer instruction
pub fn process_decompressed_token_transfer(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let instruction = TokenInstruction::unpack(&instruction_data[1..])?;
    match instruction {
        TokenInstruction::Transfer { amount } => {
            let account_infos = unsafe { convert_account_infos::<MAX_ACCOUNTS>(accounts)? };
            light_token_22::processor::Processor::process_transfer(
                &crate::ID,
                &account_infos,
                amount,
                None,
                None,
            )?;
            update_compressible_accounts_last_written_slot(account_infos.as_slice())?;
        }
        _ => return Err(ProgramError::InvalidInstructionData),
    }
    Ok(())
}

/// Update last_written_slot for token accounts with compressible extensions
/// SPL token transfer uses accounts[0] as source and accounts[1] as destination
#[inline(always)]
fn update_compressible_accounts_last_written_slot(
    accounts: &[anchor_lang::prelude::AccountInfo],
) -> Result<(), ProgramError> {
    if accounts.len() >= 2 {
        const SPL_TOKEN_ACCOUNT_SIZE: usize = 165; // Standard SPL token account size

        // Update both sender (accounts[0]) and recipient (accounts[1]) if they have extensions
        for account in &accounts[..2] {
            if account.data_len() > SPL_TOKEN_ACCOUNT_SIZE {
                if let Ok(mut account_data) = account.try_borrow_mut_data() {
                    if let Ok((mut token, _)) = CompressedToken::zero_copy_at_mut(&mut account_data)
                    {
                        token
                            .update_compressible_last_written_slot()
                            .map_err(|_| ProgramError::InvalidAccountData)?;
                    }
                }
            }
        }
    }
    Ok(())
}
