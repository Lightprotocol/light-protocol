use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_merkle_tree_state::{event_merkle_tree_from_bytes_mut, state_merkle_tree_from_bytes_mut};

/// Set of on-chain Merkle trees.
#[account(zero_copy)]
#[aligned_sized(anchor)]
pub struct MerkleTreeSet {
    /// Unique index.
    pub index: u64,
    /// Public key of the next Merkle tree set.
    pub next_merkle_tree: Pubkey,
    /// Owner of the Merkle tree set.
    pub owner: Pubkey,
    /// Merkle tree for the transaction state.
    pub state_merkle_tree: [u8; 90368],
    /// Merkle tree for event compression.
    pub event_merkle_tree: [u8; 90368],
}

impl MerkleTreeSet {
    pub fn init(&mut self, index: u64) -> Result<()> {
        self.index = index;

        state_merkle_tree_from_bytes_mut(&mut self.state_merkle_tree).init()?;
        event_merkle_tree_from_bytes_mut(&mut self.event_merkle_tree).init()?;

        Ok(())
    }
}
