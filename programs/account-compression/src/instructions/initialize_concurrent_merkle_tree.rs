use std::borrow::BorrowMut;

use anchor_lang::prelude::*;

use crate::{
    errors::AccountCompressionErrorCode, state::StateMerkleTreeAccount,
    state_mt_from_bytes_zero_copy_init,
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
    ctx: Context<InitializeStateMerkleTree>,
    index: u64,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    height: u64,
    changelog_size: u64,
    roots_size: u64,
    canopy_depth: u64,
) -> Result<()> {
    let foo = ctx.accounts.merkle_tree.to_account_info();
    // Initialize new Merkle trees.
    let mut merkle_tree = ctx.accounts.merkle_tree.load_init()?;

    merkle_tree.index = index;
    merkle_tree.owner = owner;
    merkle_tree.delegate = delegate.unwrap_or(owner);

    // TODO: think about whether and if how to use the Merkle tree index in the future
    // we could create a group which has ownership over a set of Merkle trees same registration process as for pool program
    // this needs to be the delegate and or owner
    // if part of a group we can apply the same registration model as for the pool program
    state_mt_from_bytes_zero_copy_init(
        ctx.accounts.merkle_tree,
        height as usize,
        changelog_size as usize,
        roots_size as usize,
        canopy_depth as usize,
    )?;

    Ok(())
}
