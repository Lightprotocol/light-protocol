use anchor_lang::prelude::ProgramError;
use borsh::BorshDeserialize;
use light_ctoken_types::instructions::create_associated_token_account2::CreateAssociatedTokenAccount2InstructionData;
use pinocchio::account_info::AccountInfo;

use crate::create_associated_token_account::process_create_associated_token_account_inner;

/// Process the create associated token account 2 instruction (non-idempotent)
/// Owner and mint are passed as accounts instead of instruction data
#[inline(always)]
pub fn process_create_associated_token_account2(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    process_create_associated_token_account2_inner::<false>(account_infos, instruction_data)
}

/// Process the create associated token account 2 instruction (idempotent)
/// Owner and mint are passed as accounts instead of instruction data
#[inline(always)]
pub fn process_create_associated_token_account2_idempotent(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    process_create_associated_token_account2_inner::<true>(account_infos, instruction_data)
}

/// Convert create_ata2 instruction format to create_ata format by extracting
/// owner and mint from accounts and calling the inner function directly
///
/// Account order:
/// 0. owner (non-mut, non-signer)
/// 1. mint (non-mut, non-signer)
/// 2. fee_payer (signer, mut)
/// 3. associated_token_account (mut)
/// 4. system_program
/// 5. optional accounts (config, rent_payer, etc.)
#[inline(always)]
fn process_create_associated_token_account2_inner<const IDEMPOTENT: bool>(
    account_infos: &[AccountInfo],
    mut instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if account_infos.len() < 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let instruction_inputs =
        CreateAssociatedTokenAccount2InstructionData::deserialize(&mut instruction_data)
            .map_err(ProgramError::from)?;

    let (owner_and_mint, remaining_accounts) = account_infos.split_at(2);
    let owner = &owner_and_mint[0];
    let mint = &owner_and_mint[1];

    process_create_associated_token_account_inner::<IDEMPOTENT>(
        remaining_accounts,
        owner.key(),
        mint.key(),
        instruction_inputs.bump,
        instruction_inputs.compressible_config,
    )
}
