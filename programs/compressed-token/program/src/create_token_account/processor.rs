use anchor_lang::prelude::{AccountInfo, ProgramError, SolanaSysvar};
use anchor_lang::solana_program::{
    program::invoke, pubkey::Pubkey, rent::Rent, system_instruction,
};
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

    {
        // Calculate rent for SPL token account (165 bytes)
        let token_account_size = 165_usize;
        let rent = Rent::get()?;
        let rent_lamports = rent.minimum_balance(token_account_size);

        // Create the token account
        let create_account_instruction = system_instruction::create_account(
            accounts.fee_payer.key,
            accounts.token_account.key,
            rent_lamports,
            token_account_size as u64,
            &crate::ID,
        );

        // Execute the create account instruction (no signing needed)
        invoke(
            &create_account_instruction,
            &[
                accounts.fee_payer.clone(),
                accounts.token_account.clone(),
                accounts.system_program.clone(),
            ],
        )?;
    }

    initialize_token_account(accounts.token_account, accounts.mint.key, &owner_pubkey)?;

    Ok(())
}
