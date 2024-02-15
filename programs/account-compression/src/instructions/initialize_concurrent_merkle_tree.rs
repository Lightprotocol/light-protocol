use anchor_lang::prelude::*;

use crate::state::ConcurrentMerkleTreeAccount;

#[derive(Accounts)]
pub struct InitializeConcurrentMerkleTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub merkle_tree: AccountLoader<'info, ConcurrentMerkleTreeAccount>,
    pub system_program: Program<'info, System>,
}

#[allow(unused_variables)]
pub fn process_initialize_concurrent_state_merkle_tree(
    ctx: Context<InitializeConcurrentMerkleTree>,
    index: u64,
    owner: Pubkey,
    delegate: Option<Pubkey>,
) -> Result<()> {
    // Initialize new Merkle trees.
    let mut merkle_tree = ctx.accounts.merkle_tree.load_init()?;
    // TODO: think about whether and if how to use the Merkle tree index in the future
    // we could create a group which has ownership over a set of Merkle trees same registration process as for pool program
    // this needs to be the delegate and or owner
    // if part of a group we can apply the same registration model as for the pool program
    merkle_tree.init(index)?;
    merkle_tree.owner = owner;
    merkle_tree.delegate = delegate.unwrap_or(owner);
    Ok(())
}
