use anchor_lang::prelude::*;

use crate::{event_merkle_tree_from_bytes_mut, state::MerkleTreeSet, RegisteredVerifier};

#[derive(Accounts)]
#[instruction(
    leaf_left: [u8; 32],
    leaf_right: [u8; 32],
)]
pub struct InsertTwoLeavesEvent<'info> {
    #[account(
        mut,
        seeds = [__program_id.to_bytes().as_ref()],
        bump,
        seeds::program = registered_verifier.pubkey,
    )]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub merkle_tree_set: AccountLoader<'info, MerkleTreeSet>,
    pub system_program: Program<'info, System>,
    #[account(
        seeds = [&registered_verifier.pubkey.to_bytes()],
        bump,
    )]
    pub registered_verifier: Account<'info, RegisteredVerifier>,
}

pub fn process_insert_two_leaves_event(
    ctx: Context<InsertTwoLeavesEvent>,
    leaf_left: [u8; 32],
    leaf_right: [u8; 32],
) -> Result<()> {
    let mut merkle_tree_set = ctx.accounts.merkle_tree_set.load_mut()?;
    event_merkle_tree_from_bytes_mut(&mut merkle_tree_set.event_merkle_tree)
        .append_two(&leaf_left, &leaf_right)?;

    Ok(())
}
