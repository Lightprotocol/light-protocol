use anchor_lang::prelude::*;
use light_macros::pubkey;
use light_merkle_tree::{
    config::MerkleTreeConfig,
    constants::{sha256::ZERO_BYTES, ZeroBytes},
    hasher::Sha256,
    MerkleTree,
};

use crate::utils::constants::MESSSAGE_MERKLE_TREE_SEED;

#[derive(Accounts)]
pub struct InitializeNewMessageMerkleTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: it should be unpacked internally
    #[account(
        init,
        seeds = [&program_id.to_bytes()[..], MESSSAGE_MERKLE_TREE_SEED],
        bump,
        payer = authority,
        // discriminator + height (u64) + filled subtrees ([[u8; 32]; 18]) +
        // roots ([[u8; 32]; 20]) + next_index (u64) + current_root_index (u64)
        // + hash_function (enum)
        // 8 + 8 + 18 * 32 + 20 * 32 + 8 + 8 + 8 = 1256
        space = 1256,
    )]
    pub message_merkle_tree: AccountLoader<'info, MessageMerkleTree>,
    pub system_program: Program<'info, System>,
}

#[derive(Clone, Copy)]
pub struct MessageMerkleTreeConfig {}

impl MerkleTreeConfig for MessageMerkleTreeConfig {
    const ZERO_BYTES: ZeroBytes = ZERO_BYTES;
    const PROGRAM_ID: Pubkey = pubkey!("JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6");
}

#[account(zero_copy)]
pub struct MessageMerkleTree {
    pub merkle_tree: MerkleTree<Sha256, MessageMerkleTreeConfig>,
}
