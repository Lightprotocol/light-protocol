use anchor_lang::prelude::*;

use crate::{
    errors::AccountCompressionErrorCode, initialize_address_merkle_tree::compute_rollover_fee,
    state::StateMerkleTreeAccount,
};

#[derive(Accounts)]
pub struct InitializeStateMerkleTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
    pub system_program: Program<'info, System>,
}

#[allow(unused_variables)]
pub fn process_initialize_state_merkle_tree(
    merkle_tree_account_loader: &AccountLoader<'_, StateMerkleTreeAccount>,
    index: u64,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    height: &u32,
    changelog_size: &u64,
    roots_size: &u64,
    canopy_depth: &u64,
    associated_queue: Pubkey,
    tip: u64,
    rollover_threshold: Option<u64>,
    close_threshold: Option<u64>,
    rent: u64,
) -> Result<()> {
    // Initialize new Merkle trees.
    let mut merkle_tree = merkle_tree_account_loader.load_init()?;

    merkle_tree.index = index;
    merkle_tree.owner = owner;
    merkle_tree.delegate = delegate.unwrap_or(owner);
    merkle_tree.associated_queue = associated_queue;
    merkle_tree.tip = tip;
    let total_number_of_leaves = 2u64.pow(*height);
    merkle_tree.rollover_fee = match rollover_threshold {
        Some(rollover_threshold) => {
            compute_rollover_fee(rollover_threshold, total_number_of_leaves, rent)?
        }
        None => 0,
    };
    merkle_tree.rolledover_slot = u64::MAX;
    merkle_tree.rollover_threshold = rollover_threshold.unwrap_or(u64::MAX);
    merkle_tree.close_threshold = close_threshold.unwrap_or(u64::MAX);

    // TODO: think about whether and if how to use the Merkle tree index in the future
    // we could create a group which has ownership over a set of Merkle trees same registration process as for pool program
    // this needs to be the delegate and or owner
    // if part of a group we can apply the same registration model as for the pool program
    merkle_tree
        .load_merkle_tree_init(
            (*height)
                .try_into()
                .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?,
            (*changelog_size)
                .try_into()
                .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?,
            (*roots_size)
                .try_into()
                .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?,
            (*canopy_depth)
                .try_into()
                .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?,
        )
        .map_err(ProgramError::from)?;

    Ok(())
}
