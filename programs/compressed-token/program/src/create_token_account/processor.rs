use anchor_lang::prelude::{AccountInfo, ProgramError};
use anchor_lang::solana_program::pubkey::Pubkey;
use light_zero_copy::borsh::Deserialize;

use super::{
    accounts::CreateTokenAccountAccounts, instruction_data::CreateTokenAccountInstructionData,
};
use crate::shared::initialize_token_account::initialize_token_account;

/// Process the create token account instruction
pub fn process_create_token_account<'info>(
    account_infos: &'info [AccountInfo<'info>],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse instruction data using zero-copy
    let (inputs, _) = CreateTokenAccountInstructionData::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;

    // Convert to solana pubkeys for validation
    let owner_pubkey = Pubkey::new_from_array(inputs.owner.to_bytes());

    // Validate and get accounts
    let accounts = CreateTokenAccountAccounts::get_checked(account_infos)?;

    // Initialize the token account (assumes account already exists and is owned by our program)
    initialize_token_account(accounts.token_account, accounts.mint.key, &owner_pubkey)?;

    Ok(())
}
