use anchor_lang::prelude::*;
use light_macros::pubkey;
use light_merkle_tree::{
    config::MerkleTreeConfig,
    constants::{sha256::ZERO_BYTES, ZeroBytes},
    hasher::Sha256,
    MerkleTree,
};

use crate::{
    utils::constants::{EVENT_MERKLE_TREE_SEED, MERKLE_TREE_AUTHORITY_SEED},
    MerkleTreeAuthority,
};

#[derive(Accounts)]
pub struct InitializeNewEventMerkleTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: it should be unpacked internally
    #[account(
        init,
        seeds = [
            EVENT_MERKLE_TREE_SEED,
            merkle_tree_authority_pda.event_merkle_tree_index.to_le_bytes().as_ref(),
        ],
        bump,
        payer = authority,
        // discriminator + height (u64) + filled subtrees ([[u8; 32]; 18]) +
        // roots ([[u8; 32]; 20]) + next_index (u64) + current_root_index (u64)
        // + hash_function (enum)
        // 8 + 8 + 18 * 32 + 20 * 32 + 8 + 8 + 8 + 8 = 1264
        space = 1264,
    )]
    pub event_merkle_tree: AccountLoader<'info, EventMerkleTree>,
    pub system_program: Program<'info, System>,
    #[account(mut, seeds = [MERKLE_TREE_AUTHORITY_SEED], bump)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
}

#[derive(Clone, Copy)]
pub struct EventMerkleTreeConfig {}

impl MerkleTreeConfig for EventMerkleTreeConfig {
    const ZERO_BYTES: ZeroBytes = ZERO_BYTES;
    const PROGRAM_ID: Pubkey = pubkey!("JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6");
}

#[account(zero_copy)]
pub struct EventMerkleTree {
    pub merkle_tree: MerkleTree<Sha256, EventMerkleTreeConfig>,
    pub merkle_tree_nr: u64,
}
