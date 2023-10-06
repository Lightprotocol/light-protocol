use anchor_lang::prelude::*;

use crate::{
    event_merkle_tree::EventMerkleTree,
    transaction_merkle_tree::state::TransactionMerkleTree,
    utils::constants::{
        EVENT_MERKLE_TREE_SEED, MERKLE_TREE_AUTHORITY_SEED, TRANSACTION_MERKLE_TREE_SEED,
    },
    MerkleTreeAuthority,
};

#[derive(Accounts)]
pub struct InitializeNewMerkleTrees<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        seeds = [
            TRANSACTION_MERKLE_TREE_SEED,
            merkle_tree_authority_pda.transaction_merkle_tree_index.to_le_bytes().as_ref(),
        ],
        bump,
        payer = authority,
        space = 8880 //10240 //1698
    )]
    pub new_transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
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
        // + hash_function (enum) + merkle_tree_nr (u64) + newest (u8) +
        // padding (7 * u8)
        // 8 + 8 + 18 * 32 + 20 * 32 + 8 + 8 + 8 + 8 + 8 + 1 + 7 = 1280
        space = 1280,
    )]
    pub new_event_merkle_tree: AccountLoader<'info, EventMerkleTree>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    #[account(mut, seeds = [MERKLE_TREE_AUTHORITY_SEED], bump)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
}
