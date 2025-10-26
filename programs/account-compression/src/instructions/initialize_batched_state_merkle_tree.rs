use anchor_lang::{prelude::*, solana_program::program_error::ProgramError};
use light_batched_merkle_tree::initialize_state_tree::{
    init_batched_state_merkle_tree_from_account_info, InitStateTreeAccountsInstructionData,
};

use super::RegisteredProgram;
use crate::utils::check_signer_is_registered_or_authority::{
    check_signer_is_registered_or_authority, GroupAccounts,
};

#[derive(Accounts)]
pub struct InitializeBatchedStateMerkleTreeAndQueue<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: is initialized in this instruction.
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    /// CHECK: is initialized in this instruction.
    #[account(mut)]
    pub queue: AccountInfo<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
}

impl<'info> GroupAccounts<'info> for InitializeBatchedStateMerkleTreeAndQueue<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

pub fn process_initialize_batched_state_merkle_tree<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeBatchedStateMerkleTreeAndQueue<'info>>,
    params: InitStateTreeAccountsInstructionData,
) -> Result<()> {
    #[cfg(feature = "test")]
    light_batched_merkle_tree::initialize_state_tree::validate_batched_tree_params(params);
    #[cfg(not(feature = "test"))]
    {
        use crate::errors::AccountCompressionErrorCode;
        if params != InitStateTreeAccountsInstructionData::default() {
            return err!(AccountCompressionErrorCode::UnsupportedParameters);
        }
        if let Some(registered_program_pda) = ctx.accounts.registered_program_pda.as_ref() {
            if registered_program_pda.group_authority_pda
                != pubkey!("24rt4RgeyjUCWGS2eF7L7gyNMuz6JWdqYpAvb1KRoHxs")
            {
                return err!(AccountCompressionErrorCode::UnsupportedParameters);
            }
        } else {
            return err!(AccountCompressionErrorCode::UnsupportedParameters);
        }
    }

    let owner = match ctx.accounts.registered_program_pda.as_ref() {
        Some(registered_program_pda) => {
            check_signer_is_registered_or_authority::<
                InitializeBatchedStateMerkleTreeAndQueue,
                RegisteredProgram,
            >(&ctx, registered_program_pda)?;
            registered_program_pda.group_authority_pda
        }
        None => ctx.accounts.authority.key(),
    };
    let merkle_tree_account_info = ctx.accounts.merkle_tree.to_account_info();
    let queue_account_info = ctx.accounts.queue.to_account_info();
    let additional_bytes_rent = Rent::get()?.minimum_balance(params.additional_bytes as usize);
    init_batched_state_merkle_tree_from_account_info(
        params,
        owner.into(),
        &merkle_tree_account_info,
        &queue_account_info,
        additional_bytes_rent,
    )
    .map_err(ProgramError::from)?;

    Ok(())
}
