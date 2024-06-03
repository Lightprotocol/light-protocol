use crate::{
    assert_size_equal,
    processor::{
        initialize_concurrent_merkle_tree::process_initialize_state_merkle_tree,
        initialize_nullifier_queue::process_initialize_nullifier_queue,
    },
    state::{
        queue::{queue_from_bytes_zero_copy_mut, QueueAccount},
        StateMerkleTreeAccount,
    },
    utils::{
        check_signer_is_registered_or_authority::{
            check_signer_is_registered_or_authority, GroupAccounts,
        },
        transfer_lamports::transfer_lamports,
    },
    RegisteredProgram,
};
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

#[derive(Accounts)]
pub struct RolloverStateMerkleTreeAndNullifierQueue<'info> {
    /// Signer used to pay rollover and protocol fees.
    pub fee_payer: Signer<'info>,
    /// CHECK: should only be accessed by a registered program or owner.
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    #[account(zero)]
    pub new_state_merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
    #[account(zero)]
    pub new_nullifier_queue: AccountLoader<'info, QueueAccount>,
    #[account(mut)]
    pub old_state_merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
    #[account(mut)]
    pub old_nullifier_queue: AccountLoader<'info, QueueAccount>,
}

impl<'info> GroupAccounts<'info> for RolloverStateMerkleTreeAndNullifierQueue<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

/// Checks:
/// 1. Size of new accounts matches size old accounts
/// 2. Merkle tree is ready to be rolled over
/// 3. Merkle tree and nullifier queue are associated
/// 4. Merkle tree is not already rolled over Actions:
/// 1. mark Merkle tree as rolled over in this slot
/// 2. initialize new Merkle tree and nullifier queue with the same parameters
pub fn process_rollover_state_merkle_tree_nullifier_queue_pair<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, RolloverStateMerkleTreeAndNullifierQueue<'info>>,
) -> Result<()> {
    assert_size_equal(
        &ctx.accounts.old_nullifier_queue.to_account_info(),
        &ctx.accounts.new_nullifier_queue.to_account_info(),
    )?;
    assert_size_equal(
        &ctx.accounts.old_state_merkle_tree.to_account_info(),
        &ctx.accounts.new_state_merkle_tree.to_account_info(),
    )?;
    let queue_metadata = {
        let mut merkle_tree_account_loaded = ctx.accounts.old_state_merkle_tree.load_mut()?;
        let mut queue_account_loaded = ctx.accounts.old_nullifier_queue.load_mut()?;
        check_signer_is_registered_or_authority::<
            RolloverStateMerkleTreeAndNullifierQueue,
            StateMerkleTreeAccount,
        >(&ctx, &merkle_tree_account_loaded)?;
        merkle_tree_account_loaded.metadata.rollover(
            ctx.accounts.old_nullifier_queue.key(),
            ctx.accounts.new_state_merkle_tree.key(),
        )?;
        queue_account_loaded.metadata.rollover(
            ctx.accounts.old_state_merkle_tree.key(),
            ctx.accounts.new_nullifier_queue.key(),
        )?;

        let merkle_tree_metadata = merkle_tree_account_loaded.metadata;
        let queue_metadata = queue_account_loaded.metadata;

        let merkle_tree = merkle_tree_account_loaded.load_merkle_tree_mut()?;
        let height = merkle_tree.height;

        if merkle_tree.next_index
            < ((1 << height) * merkle_tree_metadata.rollover_metadata.rollover_threshold / 100)
                as usize
        {
            return err!(crate::errors::AccountCompressionErrorCode::NotReadyForRollover);
        }

        process_initialize_state_merkle_tree(
            &ctx.accounts.new_state_merkle_tree,
            merkle_tree_metadata.rollover_metadata.index,
            merkle_tree_metadata.access_metadata.owner,
            Some(merkle_tree_metadata.access_metadata.program_owner),
            &(merkle_tree.height as u32),
            &(merkle_tree.changelog_capacity as u64),
            &(merkle_tree.roots_capacity as u64),
            &(merkle_tree.canopy_depth as u64),
            ctx.accounts.new_nullifier_queue.key(),
            merkle_tree_metadata.rollover_metadata.network_fee,
            Some(merkle_tree_metadata.rollover_metadata.rollover_threshold),
            Some(merkle_tree_metadata.rollover_metadata.close_threshold),
            ctx.accounts.new_state_merkle_tree.get_lamports(),
            ctx.accounts.new_nullifier_queue.get_lamports(),
        )?;

        queue_metadata
    };
    {
        let nullifier_queue_account = ctx.accounts.old_nullifier_queue.to_account_info();
        let mut nullifier_queue = nullifier_queue_account.try_borrow_mut_data()?;
        let nullifier_queue = unsafe { queue_from_bytes_zero_copy_mut(&mut nullifier_queue)? };
        process_initialize_nullifier_queue(
            ctx.accounts.new_nullifier_queue.to_account_info(),
            &ctx.accounts.new_nullifier_queue,
            queue_metadata.rollover_metadata.index,
            queue_metadata.access_metadata.owner,
            Some(queue_metadata.access_metadata.program_owner),
            ctx.accounts.new_state_merkle_tree.key(),
            nullifier_queue.hash_set.capacity as u16,
            nullifier_queue.hash_set.sequence_threshold as u64,
            Some(queue_metadata.rollover_metadata.rollover_threshold),
            Some(queue_metadata.rollover_metadata.close_threshold),
            queue_metadata.rollover_metadata.network_fee,
        )?;
    }
    let lamports = ctx.accounts.new_nullifier_queue.get_lamports()
        + ctx.accounts.new_state_merkle_tree.get_lamports();

    transfer_lamports(
        &ctx.accounts.old_state_merkle_tree.to_account_info(),
        &ctx.accounts.fee_payer.to_account_info(),
        lamports,
    )?;
    Ok(())
}
