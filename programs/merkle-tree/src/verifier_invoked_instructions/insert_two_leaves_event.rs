use anchor_lang::prelude::*;

use crate::{
    event_merkle_tree::EventMerkleTree, utils::constants::EVENT_MERKLE_TREE_SEED,
    RegisteredVerifier,
};

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
    #[account(mut, seeds = [
        EVENT_MERKLE_TREE_SEED,
        event_merkle_tree.load().unwrap().merkle_tree_nr.to_le_bytes().as_ref()
    ], bump)]
    pub event_merkle_tree: AccountLoader<'info, EventMerkleTree>,
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
    let mut merkle_tree = ctx.accounts.event_merkle_tree.load_mut()?;
    merkle_tree.merkle_tree.insert(leaf_left, leaf_right)?;
    Ok(())
}
