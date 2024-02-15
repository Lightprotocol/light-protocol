use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_concurrent_merkle_tree::ConcurrentMerkleTree;
use light_hasher::Poseidon;

use crate::utils::constants::{MERKLE_TREE_CHANGELOG, MERKLE_TREE_HEIGHT, MERKLE_TREE_ROOTS};

pub type StateMerkleTree =
    ConcurrentMerkleTree<Poseidon, MERKLE_TREE_HEIGHT, MERKLE_TREE_CHANGELOG, MERKLE_TREE_ROOTS>;

pub fn state_merkle_tree_from_bytes(bytes: &[u8; 90368]) -> &StateMerkleTree {
    // SAFETY: We make sure that the size of the byte slice is equal to
    // the size of `StateMerkleTree`.
    // The only reason why we are doing this is that Anchor is struggling with
    // generating IDL when `ConcurrentMerkleTree` with generics is used
    // directly as a field.
    unsafe {
        let ptr = bytes.as_ptr() as *const StateMerkleTree;
        &*ptr
    }
}

pub fn state_merkle_tree_from_bytes_mut(bytes: &mut [u8; 90368]) -> &mut StateMerkleTree {
    // SAFETY: We make sure that the size of the byte slice is equal to
    // the size of `StateMerkleTree`.
    // The only reason why we are doing this is that Anchor is struggling with
    // generating IDL when `ConcurrentMerkleTree` with generics is used
    // directly as a field.
    unsafe {
        let ptr = bytes.as_ptr() as *mut StateMerkleTree;
        &mut *ptr
    }
}

/// Concurrent state Merkle tree used for public compressed transactions.
#[account(zero_copy)]
#[aligned_sized(anchor)]
#[derive(Debug)]
pub struct ConcurrentMerkleTreeAccount {
    /// Unique index.
    pub index: u64,
    /// Public key of the next Merkle tree.
    pub next_merkle_tree: Pubkey,
    /// Owner of the Merkle tree.
    pub owner: Pubkey,
    /// Delegate of the Merkle tree.
    pub delegate: Pubkey,
    /// Merkle tree for the transaction state.
    pub state_merkle_tree: [u8; 90368],
}

impl ConcurrentMerkleTreeAccount {
    pub fn init(&mut self, index: u64) -> Result<()> {
        self.index = index;

        state_merkle_tree_from_bytes_mut(&mut self.state_merkle_tree)
            .init()
            .unwrap();

        Ok(())
    }
}
