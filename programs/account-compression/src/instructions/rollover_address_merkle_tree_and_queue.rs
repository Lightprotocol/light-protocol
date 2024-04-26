use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_macros::heap_neutral;

use crate::{
    address_queue_from_bytes_zero_copy_mut,
    initialize_address_merkle_tree::process_initialize_address_merkle_tree,
    initialize_address_queue::process_initialize_address_queue, state::AddressMerkleTreeAccount,
    transfer_lamports, utils::check_registered_or_signer::GroupAccounts, AddressQueueAccount,
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct RolloverAddressMerkleTreeAndQueue<'info> {
    /// Signer used to pay rollover and protocol fees.
    pub fee_payer: Signer<'info>,
    /// CHECK: should only be accessed by a registered program/owner/delegate.
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    #[account(zero)]
    pub new_address_merkle_tree: AccountLoader<'info, AddressMerkleTreeAccount>,
    #[account(zero)]
    pub new_queue: AccountLoader<'info, AddressQueueAccount>,
    #[account(mut)]
    pub old_address_merkle_tree: AccountLoader<'info, AddressMerkleTreeAccount>,
    #[account(mut)]
    pub old_queue: AccountLoader<'info, AddressQueueAccount>,
}

impl<'info> GroupAccounts<'info> for RolloverAddressMerkleTreeAndQueue<'info> {
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
/// 4. Rollover threshold is configured, if not tree cannot be rolled over
/// Actions:
/// 1. mark Merkle tree as rolled over in this slot
/// 2. initialize new Merkle tree and nullifier queue with the same parameters
#[heap_neutral]
pub fn process_rollover_address_merkle_tree_and_queue<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, RolloverAddressMerkleTreeAndQueue<'info>>,
) -> Result<()> {
    // TODO: revisit whether necessary
    // check_registered_or_signer::<RolloverStateMerkleTreeAndNullifierQueue, AddressMerkleTreeAccount>(
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
        let mut merkle_tree_account_loaded = ctx.accounts.old_address_merkle_tree.load_mut()?;

        let mut queue_account_loaded = ctx.accounts.old_queue.load_mut()?;

        if merkle_tree_account_loaded.rolledover_slot != u64::MAX {
            return err!(crate::errors::AccountCompressionErrorCode::MerkleTreeAlreadyRolledOver);
        }
        // assign the current slot to the rolled over slot
        let current_slot = Clock::get()?.slot;
        merkle_tree_account_loaded.rolledover_slot = current_slot;
        merkle_tree_account_loaded.next_merkle_tree = ctx.accounts.new_address_merkle_tree.key();
        queue_account_loaded.rolledover_slot = current_slot;
        queue_account_loaded.next_queue = ctx.accounts.new_queue.key();
        // check that queue_account and merkle_tree_account are associated
        if ctx.accounts.old_queue.key() != merkle_tree_account_loaded.associated_queue {
            return err!(
                crate::errors::AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated
            );
        }
        if queue_account_loaded.associated_merkle_tree != ctx.accounts.old_address_merkle_tree.key()
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
        queue_index = queue_account_loaded.index;
        queue_owner = queue_account_loaded.owner;
        queue_delegate = queue_account_loaded.delegate;
        queue_tip = queue_account_loaded.tip;

        let merkle_tree = merkle_tree_account_loaded.load_merkle_tree_mut()?;
        // Check 1: Merkle tree is ready to be rolled over
        if merkle_tree.merkle_tree.next_index
            < (2u64.pow(merkle_tree.merkle_tree.height as u32) * threshold / 100) as usize
        {
            return err!(crate::errors::AccountCompressionErrorCode::NotReadyForRollover);
        }
        height = merkle_tree.merkle_tree.height;
        process_initialize_address_merkle_tree(
            &ctx.accounts.new_address_merkle_tree,
            index,
            owner,
            Some(delegate),
            merkle_tree.merkle_tree.height as u32,
            merkle_tree.merkle_tree.changelog_capacity as u64,
            merkle_tree.merkle_tree.roots_capacity as u64,
            merkle_tree.merkle_tree.canopy_depth as u64,
            ctx.accounts.new_queue.key(),
            tip,
            rollover_threshold,
            Some(close_threshold),
            ctx.accounts.new_address_merkle_tree.get_lamports(),
        )?;
    }
    {
        let queue_account = ctx.accounts.old_queue.to_account_info();
        let mut queue = queue_account.try_borrow_mut_data()?;
        let queue = unsafe { address_queue_from_bytes_zero_copy_mut(&mut queue)? };
        process_initialize_address_queue(
            &ctx.accounts.new_queue.to_account_info(),
            &ctx.accounts.new_queue,
            queue_index,
            queue_owner,
            Some(queue_delegate),
            ctx.accounts.new_address_merkle_tree.key(),
            queue.hash_set.capacity_indices as u16,
            queue.hash_set.capacity_values as u16,
            queue.hash_set.sequence_threshold as u64,
            queue_tip,
            rollover_threshold,
            height as u32,
            ctx.accounts.new_address_merkle_tree.get_lamports(),
        )?;
    }
    let lamports =
        ctx.accounts.new_queue.get_lamports() + ctx.accounts.new_address_merkle_tree.get_lamports();

    transfer_lamports(
        &ctx.accounts.old_address_merkle_tree.to_account_info(),
        &ctx.accounts.fee_payer.to_account_info(),
        lamports,
    )?;

    Ok(())
}
