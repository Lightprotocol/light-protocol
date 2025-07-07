use anchor_lang::prelude::ProgramError;
use light_zero_copy::borsh::Deserialize;
use pinocchio::account_info::AccountInfo;

use super::{
    accounts::CreateTokenAccountAccounts, instruction_data::CreateTokenAccountInstructionData,
};
use crate::shared::initialize_token_account::initialize_token_account;

/// Process the create token account instruction
pub fn process_create_token_account<'info>(
    account_infos: &'info [AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse instruction data using zero-copy
    let (inputs, _) = CreateTokenAccountInstructionData::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;

    // Validate and get accounts
    let accounts = CreateTokenAccountAccounts::get_checked(account_infos)?;

    // Initialize the token account (assumes account already exists and is owned by our program)
    initialize_token_account(
        accounts.token_account,
        accounts.mint.key(),
        &inputs.owner.to_bytes(),
    )?;

    Ok(())
}
