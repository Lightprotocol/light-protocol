use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount,
    queue::BatchedQueueAccount,
    rollover_state_tree::{rollover_batched_state_tree, RolloverBatchStateTreeParams},
};
use light_utils::account::check_account_balance_is_rent_exempt;

use crate::{
    utils::{
        check_signer_is_registered_or_authority::{
            check_signer_is_registered_or_authority, GroupAccounts,
        },
        transfer_lamports::transfer_lamports,
    },
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct RolloverBatchedStateMerkleTree<'info> {
    #[account(mut)]
    /// Signer used to receive rollover accounts rentexemption reimbursement.
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK: is initialized in this instruction.
    #[account(zero)]
    pub new_state_merkle_tree: AccountInfo<'info>,
    /// CHECK: in state_tree_from_account_info_mut.
    #[account(mut)]
    pub old_state_merkle_tree: AccountInfo<'info>,
    /// CHECK: is initialized in this instruction.
    #[account(zero)]
    pub new_output_queue: AccountInfo<'info>,
    /// CHECK: in output_queue_from_account_info_mut.
    #[account(mut)]
    pub old_output_queue: AccountInfo<'info>,
}

impl<'info> GroupAccounts<'info> for RolloverBatchedStateMerkleTree<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

/// Rollover the old state Merkle tree and output queue to
/// new state Merkle tree and output queue.
/// 1. Check Merkle tree account discriminator, tree type, and program ownership.
/// 2. Check Queue account discriminator, and program ownership.
/// 3. Check that signer is registered or authority.
/// 4. Check that new Merkle tree account is exactly rent exempt.
/// 5. Check that new Queue account is exactly rent exempt.
/// 6. Rollover the old Merkle tree and queue to new Merkle tree and queue.
/// 7. Transfer rent exemption for new accounts
///     from old output queue to fee payer.
pub fn process_rollover_batched_state_merkle_tree<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, RolloverBatchedStateMerkleTree<'info>>,
    additional_bytes: u64,
    network_fee: Option<u64>,
) -> Result<()> {
    // 1. Check Merkle tree account discriminator, tree type, and program ownership.
    let old_merkle_tree_account = &mut BatchedMerkleTreeAccount::state_tree_from_account_info_mut(
        &ctx.accounts.old_state_merkle_tree,
    )
    .map_err(ProgramError::from)?;

    // 2. Check Queue account discriminator, and program ownership.
    let old_output_queue = &mut BatchedQueueAccount::output_queue_from_account_info_mut(
        &ctx.accounts.old_output_queue,
    )
    .map_err(ProgramError::from)?;

    // 3. Check that signer is registered or authority.
    check_signer_is_registered_or_authority::<
        RolloverBatchedStateMerkleTree,
        BatchedMerkleTreeAccount,
    >(&ctx, old_merkle_tree_account)?;

    // 4. Check that new Merkle tree account is exactly rent exempt.
    let merkle_tree_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.new_state_merkle_tree.to_account_info(),
        ctx.accounts
            .old_state_merkle_tree
            .to_account_info()
            .data_len(),
    )
    .map_err(ProgramError::from)?;
    // 5. Check that new Queue account is exactly rent exempt.
    let queue_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.new_output_queue.to_account_info(),
        ctx.accounts.old_output_queue.to_account_info().data_len(),
    )
    .map_err(ProgramError::from)?;
    let additional_bytes_rent = Rent::get()?.minimum_balance(additional_bytes as usize);
    let new_mt_data = &mut ctx.accounts.new_state_merkle_tree.try_borrow_mut_data()?;
    let params = RolloverBatchStateTreeParams {
        old_merkle_tree: old_merkle_tree_account,
        old_mt_pubkey: ctx.accounts.old_state_merkle_tree.key().into(),
        new_mt_data,
        new_mt_rent: merkle_tree_rent,
        new_mt_pubkey: ctx.accounts.new_state_merkle_tree.key().into(),
        old_output_queue,
        old_queue_pubkey: ctx.accounts.old_output_queue.key().into(),
        new_output_queue_data: &mut ctx.accounts.new_output_queue.try_borrow_mut_data()?,
        new_output_queue_rent: queue_rent,
        new_output_queue_pubkey: ctx.accounts.new_output_queue.key().into(),
        additional_bytes_rent,
        additional_bytes,
        network_fee,
    };

    // 6. Rollover the old Merkle tree and queue to new Merkle tree and queue.
    rollover_batched_state_tree(params).map_err(ProgramError::from)?;

    // 7. Transfer rent exemption for new accounts
    //     from old output queue to fee payer.
    transfer_lamports(
        &ctx.accounts.old_output_queue.to_account_info(),
        &ctx.accounts.fee_payer.to_account_info(),
        merkle_tree_rent + queue_rent + additional_bytes_rent,
    )?;

    Ok(())
}
