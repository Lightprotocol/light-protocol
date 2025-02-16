use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_account_checks::checks::check_account_balance_is_rent_exempt;
use light_merkle_tree_metadata::errors::MerkleTreeMetadataError;

use crate::{
    processor::{
        initialize_concurrent_merkle_tree::process_initialize_state_merkle_tree,
        initialize_nullifier_queue::process_initialize_nullifier_queue,
    },
    state::{
        queue::{queue_from_bytes_zero_copy_mut, QueueAccount},
        StateMerkleTreeAccount,
    },
    state_merkle_tree_from_bytes_zero_copy,
    utils::{
        check_signer_is_registered_or_authority::{
            check_signer_is_registered_or_authority, GroupAccounts,
        },
        transfer_lamports::transfer_lamports,
    },
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct RolloverStateMerkleTreeAndNullifierQueue<'info> {
    #[account(mut)]
    /// Signer used to receive rollover accounts rent exemption reimbursement.
    pub fee_payer: Signer<'info>,
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
    // TODO: rollover additional rent as well. (need to add a field to the metadata for this)
    let new_merkle_tree_account_info = ctx.accounts.new_state_merkle_tree.to_account_info();
    let merkle_tree_rent = check_account_balance_is_rent_exempt(
        &new_merkle_tree_account_info,
        ctx.accounts
            .old_state_merkle_tree
            .to_account_info()
            .data_len(),
    )
    .map_err(ProgramError::from)?;
    let new_queue_account_info = ctx.accounts.new_nullifier_queue.to_account_info();
    let queue_rent = check_account_balance_is_rent_exempt(
        &new_queue_account_info,
        ctx.accounts
            .old_nullifier_queue
            .to_account_info()
            .data_len(),
    )
    .map_err(ProgramError::from)?;

    let queue_metadata = {
        let (merkle_tree_metadata, queue_metadata) = {
            let mut merkle_tree_account_loaded = ctx.accounts.old_state_merkle_tree.load_mut()?;
            let mut queue_account_loaded = ctx.accounts.old_nullifier_queue.load_mut()?;
            check_signer_is_registered_or_authority::<
                RolloverStateMerkleTreeAndNullifierQueue,
                StateMerkleTreeAccount,
            >(&ctx, &merkle_tree_account_loaded)?;
            merkle_tree_account_loaded
                .metadata
                .rollover(
                    ctx.accounts.old_nullifier_queue.key().into(),
                    ctx.accounts.new_state_merkle_tree.key().into(),
                )
                .map_err(ProgramError::from)?;
            queue_account_loaded
                .metadata
                .rollover(
                    ctx.accounts.old_state_merkle_tree.key().into(),
                    ctx.accounts.new_nullifier_queue.key().into(),
                )
                .map_err(ProgramError::from)?;

            let merkle_tree_metadata = merkle_tree_account_loaded.metadata;
            let queue_metadata = queue_account_loaded.metadata;

            (merkle_tree_metadata, queue_metadata)
        };

        let merkle_tree = ctx.accounts.old_state_merkle_tree.to_account_info();
        let merkle_tree = merkle_tree.try_borrow_data()?;
        let merkle_tree = state_merkle_tree_from_bytes_zero_copy(&merkle_tree)?;

        let height = merkle_tree.height;

        if merkle_tree.next_index()
            < ((1 << height) * merkle_tree_metadata.rollover_metadata.rollover_threshold / 100)
                as usize
        {
            return err!(crate::errors::AccountCompressionErrorCode::NotReadyForRollover);
        }

        process_initialize_state_merkle_tree(
            &ctx.accounts.new_state_merkle_tree,
            merkle_tree_metadata.rollover_metadata.index,
            merkle_tree_metadata.access_metadata.owner.into(),
            Some(
                merkle_tree_metadata
                    .access_metadata
                    .program_owner
                    .to_bytes()
                    .into(),
            ),
            Some(
                merkle_tree_metadata
                    .access_metadata
                    .forester
                    .to_bytes()
                    .into(),
            ),
            &(merkle_tree.height as u32),
            &(merkle_tree.changelog.capacity() as u64),
            &(merkle_tree.roots.capacity() as u64),
            &(merkle_tree.canopy_depth as u64),
            ctx.accounts.new_nullifier_queue.key(),
            merkle_tree_metadata.rollover_metadata.network_fee,
            Some(merkle_tree_metadata.rollover_metadata.rollover_threshold),
            Some(merkle_tree_metadata.rollover_metadata.close_threshold),
            merkle_tree_rent,
            queue_rent,
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
            queue_metadata.access_metadata.owner.into(),
            Some(
                queue_metadata
                    .access_metadata
                    .program_owner
                    .to_bytes()
                    .into(),
            ),
            Some(queue_metadata.access_metadata.forester.into()),
            ctx.accounts.new_state_merkle_tree.key(),
            nullifier_queue.hash_set.get_capacity() as u16,
            nullifier_queue.hash_set.sequence_threshold as u64,
            Some(queue_metadata.rollover_metadata.rollover_threshold),
            Some(queue_metadata.rollover_metadata.close_threshold),
            queue_metadata.rollover_metadata.network_fee,
        )?;
    }
    let lamports = merkle_tree_rent + queue_rent;

    transfer_lamports(
        &ctx.accounts.old_state_merkle_tree.to_account_info(),
        &ctx.accounts.fee_payer.to_account_info(),
        lamports,
    )?;
    if ctx
        .accounts
        .old_state_merkle_tree
        .to_account_info()
        .lamports()
        == 0
    {
        return Err(ProgramError::from(MerkleTreeMetadataError::NotReadyForRollover).into());
    }
    Ok(())
}
