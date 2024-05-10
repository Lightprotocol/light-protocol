use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_macros::heap_neutral;

use crate::{
    processor::{
        initialize_concurrent_merkle_tree::process_initialize_state_merkle_tree,
        initialize_nullifier_queue::{
            nullifier_queue_from_bytes_zero_copy_mut, process_initialize_nullifier_queue,
            NullifierQueueAccount,
        },
    },
    state::StateMerkleTreeAccount,
    transfer_lamports,
    utils::check_registered_or_signer::GroupAccounts,
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct RolloverStateMerkleTreeAndNullifierQueue<'info> {
    /// Signer used to pay rollover and protocol fees.
    pub fee_payer: Signer<'info>,
    /// CHECK: should only be accessed by a registered program/owner/delegate.
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    #[account(zero)]
    pub new_state_merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
    #[account(zero)]
    pub new_nullifier_queue: AccountLoader<'info, NullifierQueueAccount>,
    #[account(mut)]
    pub old_state_merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
    #[account(mut)]
    pub old_nullifier_queue: AccountLoader<'info, NullifierQueueAccount>,
}

impl<'info> GroupAccounts<'info> for RolloverStateMerkleTreeAndNullifierQueue<'info> {
    fn get_signing_address(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

/// Checks:
/// 1. Merkle tree is ready to be rolled over
/// 2. Merkle tree and nullifier queue are associated
/// 3. Merkle tree is not already rolled over
/// Actions:
/// 1. mark Merkle tree as rolled over in this slot
/// 2. initialize new Merkle tree and nullifier queue with the same parameters
#[heap_neutral]
pub fn process_rollover_state_merkle_tree_nullifier_queue_pair<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, RolloverStateMerkleTreeAndNullifierQueue<'info>>,
) -> Result<()> {
    // TODO: revisit whether necessary
    // check_registered_or_signer::<RolloverStateMerkleTreeAndNullifierQueue, StateMerkleTreeAccount>(
    //     &ctx,
    //     &merkle_tree,
    // )?;
    let index;
    let owner;
    let delegate;
    let threshold;
    let tip;
    let rollover_threshold;
    let close_threshold;
    let queue_index;
    let queue_owner;
    let queue_delegate;
    let queue_tip;
    let height;
    {
        let mut merkle_tree_account_loaded = ctx.accounts.old_state_merkle_tree.load_mut()?;

        let mut nullifier_queue_account_loaded = ctx.accounts.old_nullifier_queue.load_mut()?;

        if merkle_tree_account_loaded.rolledover_slot != u64::MAX {
            return err!(crate::errors::AccountCompressionErrorCode::MerkleTreeAlreadyRolledOver);
        }
        // assign the current slot to the rolled over slot
        let current_slot = Clock::get()?.slot;
        merkle_tree_account_loaded.rolledover_slot = current_slot;
        merkle_tree_account_loaded.next_merkle_tree = ctx.accounts.new_state_merkle_tree.key();
        nullifier_queue_account_loaded.rolledover_slot = current_slot;
        nullifier_queue_account_loaded.next_queue = ctx.accounts.new_nullifier_queue.key();
        // check that nullifier_queue_account and merkle_tree_account are associated
        if ctx.accounts.old_nullifier_queue.key() != merkle_tree_account_loaded.associated_queue {
            return err!(
                crate::errors::AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated
            );
        }
        if nullifier_queue_account_loaded.associated_merkle_tree
            != ctx.accounts.old_state_merkle_tree.key()
        {
            return err!(
                crate::errors::AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated
            );
        }
        index = merkle_tree_account_loaded.index;
        owner = merkle_tree_account_loaded.owner;
        delegate = merkle_tree_account_loaded.delegate;
        threshold = merkle_tree_account_loaded.rollover_threshold;
        tip = merkle_tree_account_loaded.tip;
        rollover_threshold = if merkle_tree_account_loaded.rollover_threshold != 0 {
            Some(merkle_tree_account_loaded.rollover_threshold)
        } else {
            return err!(crate::errors::AccountCompressionErrorCode::RolloverNotConfigured);
        };
        close_threshold = merkle_tree_account_loaded.close_threshold;
        queue_index = nullifier_queue_account_loaded.index;
        queue_owner = nullifier_queue_account_loaded.owner;
        queue_delegate = nullifier_queue_account_loaded.delegate;
        queue_tip = nullifier_queue_account_loaded.tip;
        let merkle_tree = merkle_tree_account_loaded.load_merkle_tree_mut()?;
        // Check 1: Merkle tree is ready to be rolled over
        if merkle_tree.next_index < (2u64.pow(merkle_tree.height as u32) * threshold / 100) as usize
        {
            return err!(crate::errors::AccountCompressionErrorCode::NotReadyForRollover);
        }
        height = merkle_tree.height;

        process_initialize_state_merkle_tree(
            &ctx.accounts.new_state_merkle_tree,
            index,
            owner,
            Some(delegate),
            &(merkle_tree.height as u32),
            &(merkle_tree.changelog.capacity() as u64),
            &(merkle_tree.roots.capacity() as u64),
            &(merkle_tree.canopy_depth as u64),
            ctx.accounts.new_nullifier_queue.key(),
            tip,
            rollover_threshold,
            Some(close_threshold),
            ctx.accounts.new_state_merkle_tree.get_lamports(),
        )?;
    }
    {
        let nullifier_queue_account = ctx.accounts.old_nullifier_queue.to_account_info();
        let mut nullifier_queue = nullifier_queue_account.try_borrow_mut_data()?;
        let nullifier_queue =
            unsafe { nullifier_queue_from_bytes_zero_copy_mut(&mut nullifier_queue)? };
        process_initialize_nullifier_queue(
            ctx.accounts.new_nullifier_queue.to_account_info(),
            &ctx.accounts.new_nullifier_queue,
            queue_index,
            queue_owner,
            Some(queue_delegate),
            ctx.accounts.new_state_merkle_tree.key(),
            nullifier_queue.hash_set.capacity_indices as u16,
            nullifier_queue.hash_set.capacity_values as u16,
            nullifier_queue.hash_set.sequence_threshold as u64,
            rollover_threshold,
            queue_tip,
            height as u32,
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
