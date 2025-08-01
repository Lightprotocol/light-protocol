use anchor_lang::prelude::ProgramError;
use light_zero_copy::borsh::Deserialize;
use pinocchio::account_info::AccountInfo;

use super::{
    accounts::CreateTokenAccountAccounts, instruction_data::CreateTokenAccountInstructionData,
};
use crate::shared::initialize_token_account::initialize_token_account;

/// Process the create token account instruction
pub fn process_create_token_account(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let mut backup_instruction_data = [0u8; 33];
    let (inputs, _) = if instruction_data.len() == 32 {
        // Extend instruction data with a zero option byte for initialize_3 spl_token instruction compatibility
        backup_instruction_data[0..32].copy_from_slice(instruction_data);
        CreateTokenAccountInstructionData::zero_copy_at(backup_instruction_data.as_slice())
            .map_err(ProgramError::from)?
    } else {
        CreateTokenAccountInstructionData::zero_copy_at(instruction_data)
            .map_err(ProgramError::from)?
    };

    // Validate and get accounts
    let accounts = CreateTokenAccountAccounts::validate_and_parse(account_infos)?;

    // Initialize the token account (assumes account already exists and is owned by our program)
    initialize_token_account(
        accounts.token_account,
        accounts.mint.key(),
        &inputs.owner.to_bytes(),
        inputs.compressible_config,
    )?;

    Ok(())
}
