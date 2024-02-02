use anchor_lang::prelude::*;

use crate::{
    errors::ErrorCode, state::MerkleTreeSet, utils::constants::MERKLE_TREE_AUTHORITY_SEED,
    MerkleTreeAuthority,
};

#[derive(Accounts)]
pub struct InitializeNewMerkleTreeSet<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub new_merkle_tree_set: AccountLoader<'info, MerkleTreeSet>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    #[account(mut, seeds = [MERKLE_TREE_AUTHORITY_SEED], bump)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
}

#[allow(unused_variables)]
pub fn process_initialize_new_merkle_tree_set(
    ctx: Context<InitializeNewMerkleTreeSet>,
) -> Result<()> {
    if !ctx
        .accounts
        .merkle_tree_authority_pda
        .enable_permissionless_merkle_tree_registration
        && ctx.accounts.authority.key() != ctx.accounts.merkle_tree_authority_pda.pubkey
    {
        return err!(ErrorCode::InvalidAuthority);
    }

    let merkle_tree_authority = &mut ctx.accounts.merkle_tree_authority_pda;

    // Initialize new Merkle trees.
    let mut new_merkle_trees = ctx.accounts.new_merkle_tree_set.load_init()?;
    new_merkle_trees.init(merkle_tree_authority.merkle_tree_set_index)?;
    merkle_tree_authority.merkle_tree_set_index += 1;

    Ok(())
}
