use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_ctoken_types::state::CompressedToken;
use light_profiler::profile;
use light_zero_copy::traits::ZeroCopyAtMut;
use pinocchio::account_info::AccountInfo;
use spl_token::instruction::TokenInstruction;

use crate::{convert_account_infos::convert_account_infos, MAX_ACCOUNTS};

/// Process decompressed token transfer instruction
#[profile]
pub fn process_decompressed_token_transfer(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if accounts.len() != 3 {
        msg!(
            "Decompressed transfer: expected 3 accounts received {}",
            accounts.len()
        );
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let instruction = TokenInstruction::unpack(&instruction_data[1..])?;
    match instruction {
        TokenInstruction::Transfer { amount } => {
            let account_infos = unsafe { convert_account_infos::<MAX_ACCOUNTS>(accounts)? };

            process_light_token_transfer(&crate::ID, &account_infos, amount)?;
            update_compressible_accounts_last_written_slot(account_infos.as_slice())?;
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
) -> Result<(), ProgramError> {
    // Update sender (accounts[0]) and recipient (accounts[1])
    // if these have extensions.
    for account in &accounts[..2] {
        if account.data_len() > light_ctoken_types::BASE_TOKEN_ACCOUNT_SIZE as usize {
            let mut account_data = account.try_borrow_mut_data()?;
            let (mut token, _) = CompressedToken::zero_copy_at_mut(&mut account_data)?;
            token.update_compressible_last_written_slot()?;
        }
    }
    Ok(())
}
