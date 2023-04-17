use anchor_lang::prelude::*;

use crate::{message_merkle_tree::MessageMerkleTree, utils::constants::MESSSAGE_MERKLE_TREE_SEED};

#[derive(Accounts)]
#[instruction(
    leaf_left: [u8; 32],
    leaf_right: [u8; 32],
)]
pub struct InsertTwoLeavesMessage<'info> {
    #[account(mut, seeds = [&program_id.to_bytes()[..], MESSSAGE_MERKLE_TREE_SEED], bump)]
    pub message_merkle_tree: AccountLoader<'info, MessageMerkleTree>,
    pub system_program: Program<'info, System>,
}

pub fn process_insert_two_leaves_message(
    ctx: Context<InsertTwoLeavesMessage>,
    leaf_left: [u8; 32],
    leaf_right: [u8; 32],
) -> Result<()> {
    let mut merkle_tree = ctx.accounts.message_merkle_tree.load_mut()?;
    merkle_tree.merkle_tree.insert(leaf_left, leaf_right);
    Ok(())
}
