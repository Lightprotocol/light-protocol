use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_hasher::{Poseidon, Sha256};
use light_macros::pubkey;
use light_sparse_merkle_tree::{config::MerkleTreeConfig, HashFunction, MerkleTree};

use crate::utils::config::MERKLE_TREE_HEIGHT;

#[derive(Clone, Copy)]
pub struct StateMerkleTreeConfig {}

impl MerkleTreeConfig for StateMerkleTreeConfig {
    const PROGRAM_ID: Pubkey = pubkey!("JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6");
}

#[derive(Clone, Copy)]
pub struct EventMerkleTreeConfig {}

impl MerkleTreeConfig for EventMerkleTreeConfig {
    const PROGRAM_ID: Pubkey = pubkey!("JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6");
}

/// Set of on-chain Merkle trees.
#[account(zero_copy)]
#[aligned_sized(anchor)]
pub struct MerkleTreeSet {
    /// Unique index.
    pub index: u64,
    /// Merkle tree for the transaction state.
    pub state_merkle_tree: MerkleTree<Poseidon, StateMerkleTreeConfig>,
    /// Merkle tree for event compression.
    pub event_merkle_tree: MerkleTree<Sha256, EventMerkleTreeConfig>,
    /// Public key of the next Merkle tree set.
    pub next_merkle_tree: Pubkey,
    /// Owner of the Merkle tree set.
    pub owner: Pubkey,
}

impl MerkleTreeSet {
    pub fn init(&mut self, index: u64) -> Result<()> {
        self.index = index;
        self.state_merkle_tree
            .init(MERKLE_TREE_HEIGHT, HashFunction::Poseidon)?;
        self.event_merkle_tree
            .init(MERKLE_TREE_HEIGHT, HashFunction::Sha256)?;
        Ok(())
    }
}
