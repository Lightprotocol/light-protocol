use std::cell::RefMut;

use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{transaction_merkle_tree::state::TransactionMerkleTree, MerkleTreeAuthority};

// keeps track of leaves which have been queued but not inserted into the merkle tree yet
#[account]
pub struct MerkleTreePdaToken {}

// keeps track of leaves which have been queued but not inserted into the merkle tree yet
#[account]
pub struct PreInsertedLeavesIndex {
    pub next_index: u64,
}

pub fn process_initialize_new_merkle_tree(
    merkle_tree: &mut RefMut<'_, TransactionMerkleTree>,
    merkle_tree_authority: &mut Account<'_, MerkleTreeAuthority>,
    height: usize,
) -> Result<()> {
    use light_sparse_merkle_tree::HashFunction;

    merkle_tree.newest = 1;
    merkle_tree
        .merkle_tree
        .init(height, HashFunction::Poseidon)?;
    merkle_tree_authority.transaction_merkle_tree_index += 1;

    Ok(())
}
