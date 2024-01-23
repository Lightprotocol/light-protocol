use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_concurrent_merkle_tree::ConcurrentMerkleTree;
use light_hasher::{Poseidon, Sha256};

use crate::utils::config::{MERKLE_TREE_CHANGELOG, MERKLE_TREE_HEIGHT, MERKLE_TREE_ROOTS};

/// Set of on-chain Merkle trees.
#[account(zero_copy)]
#[aligned_sized(anchor)]
pub struct MerkleTreeSet {
    /// Unique index.
    pub index: u64,
    /// Merkle tree for the transaction state.
    pub state_merkle_tree: ConcurrentMerkleTree<
        Poseidon,
        MERKLE_TREE_HEIGHT,
        MERKLE_TREE_CHANGELOG,
        MERKLE_TREE_ROOTS,
    >,
    /// Merkle tree for event compression.
    pub event_merkle_tree:
        ConcurrentMerkleTree<Sha256, MERKLE_TREE_HEIGHT, MERKLE_TREE_CHANGELOG, MERKLE_TREE_ROOTS>,
    /// Public key of the next Merkle tree set.
    pub next_merkle_tree: Pubkey,
    /// Owner of the Merkle tree set.
    pub owner: Pubkey,
}

impl MerkleTreeSet {
    pub fn init(&mut self, index: u64) -> Result<()> {
        self.index = index;
        self.state_merkle_tree.init()?;
        self.event_merkle_tree.init()?;
        Ok(())
    }
}
