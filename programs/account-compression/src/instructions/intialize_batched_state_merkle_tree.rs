use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_error::ProgramError;
use light_batched_merkle_tree::{
    initialize_state_tree::{
        init_batched_state_merkle_tree_accounts, validate_batched_tree_params,
        InitStateTreeAccountsInstructionData,
    },
    merkle_tree::{get_merkle_tree_account_size, BatchedMerkleTreeAccount},
    queue::{get_output_queue_account_size, BatchedQueueAccount},
    zero_copy::check_account_info_init,
};

use crate::utils::{
    check_account::check_account_balance_is_rent_exempt,
    check_signer_is_registered_or_authority::{
        check_signer_is_registered_or_authority, GroupAccounts,
    },
};

use super::RegisteredProgram;

#[derive(Accounts)]
pub struct InitializeBatchedStateMerkleTreeAndQueue<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: is initialized in this instruction.
    #[account(zero)]
    pub merkle_tree: AccountInfo<'info>,
    /// CHECK: is initialized in this instruction.
    #[account(zero)]
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
    validate_batched_tree_params(params);
    #[cfg(not(feature = "test"))]
    {
        if params != InitStateTreeAccountsInstructionData::default() {
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

    let output_queue_pubkey = ctx.accounts.queue.key();
    let queue_account_size = get_output_queue_account_size(
        params.output_queue_batch_size,
        params.output_queue_zkp_batch_size,
        params.output_queue_num_batches,
    );
    let mt_account_size = get_merkle_tree_account_size(
        params.input_queue_batch_size,
        params.bloom_filter_capacity,
        params.input_queue_zkp_batch_size,
        params.root_history_capacity,
        params.height,
        params.input_queue_num_batches,
    );

    let queue_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.queue.to_account_info(),
        queue_account_size,
    )?;

    let mt_pubkey = ctx.accounts.merkle_tree.key();
    let merkle_tree_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.merkle_tree.to_account_info(),
        mt_account_size,
    )?;

    let additional_bytes_rent = (Rent::get()?).minimum_balance(params.additional_bytes as usize);

    check_account_info_init::<BatchedQueueAccount>(crate::ID, &ctx.accounts.queue)
        .map_err(ProgramError::from)?;
    let output_queue_account_data: AccountInfo<'info> = ctx.accounts.queue.to_account_info();
    let queue_data = &mut output_queue_account_data.try_borrow_mut_data()?;

    check_account_info_init::<BatchedMerkleTreeAccount>(crate::ID, &ctx.accounts.merkle_tree)
        .map_err(ProgramError::from)?;
    let mt_account_info = ctx.accounts.merkle_tree.to_account_info();
    let mt_data = &mut mt_account_info.try_borrow_mut_data()?;
    init_batched_state_merkle_tree_accounts(
        owner,
        params,
        queue_data,
        output_queue_pubkey,
        queue_rent,
        mt_data,
        mt_pubkey,
        merkle_tree_rent,
        additional_bytes_rent,
    )
    .map_err(ProgramError::from)?;

    Ok(())
}
