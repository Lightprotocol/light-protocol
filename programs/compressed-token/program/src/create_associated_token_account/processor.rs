use anchor_lang::prelude::ProgramError;
use light_ctoken_types::instructions::create_associated_token_account::CreateAssociatedTokenAccountInstructionData;
use light_zero_copy::borsh::Deserialize;
use pinocchio::account_info::AccountInfo;

use super::accounts::CreateAssociatedTokenAccountAccounts;
use crate::shared::initialize_token_account::initialize_token_account;

/// Note:
/// - we don't validate the mint because it would be very expensive with compressed mints
/// - it is possible to create an associated token account for non existing mints
/// - accounts with non existing mints can never have a balance
///   Process the create associated token account instruction
pub fn process_create_associated_token_account(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse instruction data using zero-copy
    let (inputs, _) = CreateAssociatedTokenAccountInstructionData::zero_copy_at(instruction_data)
        .map_err(ProgramError::from)?;

    // Validate and get accounts
    let accounts = CreateAssociatedTokenAccountAccounts::validate_and_parse(
        account_infos,
        &inputs.mint.to_bytes(),
        false,
    )?;

    // Create the associated token account using shared function
    {
        let owner = inputs.owner.to_bytes();
        let mint = inputs.mint.to_bytes();
        
        // Calculate account size based on whether compressible extension is needed
        let token_account_size = if inputs.compressible_config.is_some() {
            light_ctoken_types::COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize
        } else {
            light_ctoken_types::BASIC_TOKEN_ACCOUNT_SIZE as usize
        };

        let seeds = &[
            owner.as_ref(),
            crate::ID.as_ref(),
            mint.as_ref(),
        ];

        let config = crate::shared::CreatePdaAccountConfig {
            seeds,
            bump: inputs.bump,
            account_size: token_account_size,
            owner_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
            derivation_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
        };

        crate::shared::create_pda_account(
            accounts.fee_payer,
            accounts.associated_token_account,
            accounts.system_program,
            config,
        )?;
    }

    // Initialize the token account using shared utility
    initialize_token_account(
        accounts.associated_token_account,
        &inputs.mint.to_bytes(),
        &inputs.owner.to_bytes(),
        inputs.compressible_config,
    )?;

    Ok(())
}
