use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountIterator;
use light_ctoken_types::instructions::create_associated_token_account::CreateAssociatedTokenAccountInstructionData;
use light_zero_copy::traits::ZeroCopyAt;
use pinocchio::account_info::AccountInfo;

use crate::shared::{
    create_pda_account, initialize_token_account::initialize_token_account,
    validate_ata_derivation, CreatePdaAccountConfig,
};

/// Process the create associated token account instruction (non-idempotent)
#[inline(always)]
pub fn process_create_associated_token_account(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    process_create_associated_token_account_with_mode::<false>(account_infos, instruction_data)
}

/// Process the create associated token account instruction (non-idempotent)
#[inline(always)]
pub fn process_create_associated_token_account_idempotent(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    process_create_associated_token_account_with_mode::<true>(account_infos, instruction_data)
}

/// Process create associated token account with compile-time idempotent mode
///
/// Note:
/// - we don't validate the mint because it would be very expensive with compressed mints
/// - it is possible to create an associated token account for non existing mints
/// - accounts with non existing mints can never have a balance
#[inline(always)]
fn process_create_associated_token_account_with_mode<const IDEMPOTENT: bool>(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let (instruction_inputs, _) =
        CreateAssociatedTokenAccountInstructionData::zero_copy_at(instruction_data)
            .map_err(ProgramError::from)?;
    let mut iter = AccountIterator::new(account_infos);

    let fee_payer = iter.next_signer_mut("fee_payer")?;
    let associated_token_account = iter.next_mut("associated_token_account")?;
    let system_program = iter.next_non_mut("system_program")?;

    let owner_bytes = instruction_inputs.owner.to_bytes();
    let mint_bytes = instruction_inputs.mint.to_bytes();

    // If idempotent mode, check if account already exists
    if IDEMPOTENT {
        // Verify the PDA derivation is correct
        validate_ata_derivation(
            associated_token_account,
            &owner_bytes,
            &mint_bytes,
            instruction_inputs.bump,
        )?;
        // If account is already owned by our program, it exists - return success
        if associated_token_account.is_owned_by(&crate::LIGHT_CPI_SIGNER.program_id) {
            return Ok(());
        }
    }

    // Check account is owned by system program (uninitialized)
    if !associated_token_account.is_owned_by(&[0u8; 32]) {
        return Err(ProgramError::IllegalOwner);
    }

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

    let config = CreatePdaAccountConfig {
        seeds,
        bump: instruction_inputs.bump,
        account_size: token_account_size,
        owner_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
        derivation_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
    };

    create_pda_account(fee_payer, associated_token_account, system_program, config)?;

    initialize_token_account(
        associated_token_account,
        &mint_bytes,
        &owner_bytes,
        instruction_inputs.compressible_config,
    )?;

    Ok(())
}
