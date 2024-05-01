use anchor_lang::prelude::*;
use light_utils::fee::compute_rollover_fee;

use crate::{
    errors::AccountCompressionErrorCode, state::StateMerkleTreeAccount, AccessMetadata,
    RolloverMetadata,
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
    network_fee: u64,
    rollover_threshold: Option<u64>,
    close_threshold: Option<u64>,
    rent: u64,
    queue_rent: u64,
) -> Result<()> {
    // Initialize new Merkle trees.
    let mut merkle_tree = merkle_tree_account_loader.load_init()?;

    let rollover_fee = match rollover_threshold {
        Some(rollover_threshold) => {
            compute_rollover_fee(rollover_threshold, *height, rent).map_err(ProgramError::from)?
                + compute_rollover_fee(rollover_threshold, *height, queue_rent)
                    .map_err(ProgramError::from)?
        }
        None => 0,
    };

    merkle_tree.init(
        AccessMetadata::new(owner, delegate),
        RolloverMetadata::new(
            index,
            rollover_fee,
            rollover_threshold,
            network_fee,
            close_threshold,
        ),
        associated_queue,
    );

    // TODO: think about whether and if yes how to use the Merkle tree index in
    // the future. We could create a group which has ownership over a set of
    // Merkle trees; same registration process as for pool program this needs to
    // be the delegate and or owner. If part of a group we can apply the same
    // registration model as for the pool program.
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
