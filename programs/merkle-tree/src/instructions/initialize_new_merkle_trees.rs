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
        space = TransactionMerkleTree::LEN,
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
        space = EventMerkleTree::LEN,
    )]
    pub new_event_merkle_tree: AccountLoader<'info, EventMerkleTree>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    #[account(mut, seeds = [MERKLE_TREE_AUTHORITY_SEED], bump)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
}
