use crate::{
    utils::{
        check_account::check_account_balance_is_rent_exempt,
        check_signer_is_registered_or_authority::{
            check_signer_is_registered_or_authority, GroupAccounts,
        },
        transfer_lamports::transfer_lamports,
    },
    RegisteredProgram,
};
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_batched_merkle_tree::{
    merkle_tree::ZeroCopyBatchedMerkleTreeAccount,
    queue::ZeroCopyBatchedQueueAccount,
    rollover_state_tree::{rollover_batch_state_tree, RolloverBatchStateTreeParams},
};

#[derive(Accounts)]
pub struct RolloverBatchStateMerkleTree<'info> {
    #[account(mut)]
    /// Signer used to receive rollover accounts rentexemption reimbursement.
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK: is initialized in this instruction.
    #[account(zero)]
    pub new_state_merkle_tree: AccountInfo<'info>,
    /// CHECK: checked in manual deserialization.
    #[account(mut)]
    pub old_state_merkle_tree: AccountInfo<'info>,
    /// CHECK: is initialized in this instruction.
    #[account(zero)]
    pub new_output_queue: AccountInfo<'info>,
    /// CHECK: checked in manual deserialization.
    #[account(mut)]
    pub old_output_queue: AccountInfo<'info>,
}

impl<'info> GroupAccounts<'info> for RolloverBatchStateMerkleTree<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

/// Checks:
/// 1. Merkle tree is ready to be rolled over
/// 2. Merkle tree is not already rolled over
/// 3. Rollover threshold is configured, if not tree cannot be rolled over
///
/// Actions:
/// 1. mark Merkle tree as rolled over in this slot
/// 2. initialize new Merkle tree and output queue with the same parameters
pub fn process_rollover_batch_state_merkle_tree<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, RolloverBatchStateMerkleTree<'info>>,
    additional_bytes: u64,
    network_fee: Option<u64>,
) -> Result<()> {
    let old_merkle_tree_account =
        &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_account_info_mut(
            &ctx.accounts.old_state_merkle_tree,
        )
        .map_err(ProgramError::from)?;
    let old_output_queue = &mut ZeroCopyBatchedQueueAccount::output_queue_from_account_info_mut(
        &ctx.accounts.old_output_queue,
    )
    .map_err(ProgramError::from)?;
    check_signer_is_registered_or_authority::<
        RolloverBatchStateMerkleTree,
        ZeroCopyBatchedMerkleTreeAccount,
    >(&ctx, old_merkle_tree_account)?;

    let merkle_tree_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.new_state_merkle_tree.to_account_info(),
        ctx.accounts
            .old_state_merkle_tree
            .to_account_info()
            .data_len(),
    )?;
    let queue_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.new_output_queue.to_account_info(),
        ctx.accounts.old_output_queue.to_account_info().data_len(),
    )?;
    let additional_bytes_rent = Rent::get()?.minimum_balance(additional_bytes as usize);
    let new_mt_data = &mut ctx.accounts.new_state_merkle_tree.try_borrow_mut_data()?;
    let params = RolloverBatchStateTreeParams {
        old_merkle_tree: old_merkle_tree_account,
        old_mt_pubkey: ctx.accounts.old_state_merkle_tree.key(),
        new_mt_data,
        new_mt_rent: merkle_tree_rent,
        new_mt_pubkey: ctx.accounts.new_state_merkle_tree.key(),
        old_output_queue,
        old_queue_pubkey: ctx.accounts.old_output_queue.key(),
        new_output_queue_data: &mut ctx.accounts.new_output_queue.try_borrow_mut_data()?,
        new_output_queue_rent: queue_rent,
        new_output_queue_pubkey: ctx.accounts.new_output_queue.key(),
        additional_bytes_rent,
        additional_bytes,
        network_fee,
    };

    rollover_batch_state_tree(params).map_err(ProgramError::from)?;

    transfer_lamports(
        &ctx.accounts.old_output_queue.to_account_info(),
        &ctx.accounts.fee_payer.to_account_info(),
        merkle_tree_rent + queue_rent + additional_bytes_rent,
    )?;

    Ok(())
}
