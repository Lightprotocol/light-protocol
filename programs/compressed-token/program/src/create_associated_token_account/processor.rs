use anchor_lang::prelude::ProgramError;
use light_ctoken_types::instructions::create_associated_token_account::CreateAssociatedTokenAccountInstructionData;
use light_zero_copy::traits::ZeroCopyAt;
use pinocchio::account_info::AccountInfo;

use super::accounts::CreateAssociatedTokenAccountAccounts;
use crate::shared::initialize_token_account::initialize_token_account;

/// Process the create associated token account instruction
///
/// Note:
/// - we don't validate the mint because it would be very expensive with compressed mints
/// - it is possible to create an associated token account for non existing mints
/// - accounts with non existing mints can never have a balance
pub fn process_create_associated_token_account(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let (instruction_inputs, _) =
        CreateAssociatedTokenAccountInstructionData::zero_copy_at(instruction_data)
            .map_err(ProgramError::from)?;

    let owner_bytes = instruction_inputs.owner.to_bytes();
    let mint_bytes = instruction_inputs.mint.to_bytes();

    let accounts = CreateAssociatedTokenAccountAccounts::validate_and_parse(
        account_infos,
        &mint_bytes,
        false,
    )?;

    let token_account_size = if instruction_inputs.compressible_config.is_some() {
        light_ctoken_types::COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize
    } else {
        light_ctoken_types::BASE_TOKEN_ACCOUNT_SIZE as usize
    };

    let seeds = &[
        owner_bytes.as_ref(),
        crate::LIGHT_CPI_SIGNER.program_id.as_ref(),
        mint_bytes.as_ref(),
    ];

    let config = crate::shared::CreatePdaAccountConfig {
        seeds,
        bump: instruction_inputs.bump,
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

    initialize_token_account(
        accounts.associated_token_account,
        &mint_bytes,
        &owner_bytes,
        instruction_inputs.compressible_config,
    )?;

    Ok(())
}
