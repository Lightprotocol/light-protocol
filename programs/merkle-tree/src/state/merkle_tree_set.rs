use aligned_sized::aligned_sized;
use anchor_lang::prelude::*;
use light_concurrent_merkle_tree::ConcurrentMerkleTree22;
use light_hasher::{Poseidon, Sha256};

pub type StateMerkleTree<'a> = ConcurrentMerkleTree22<'a, Poseidon>;

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

pub type EventMerkleTree<'a> = ConcurrentMerkleTree22<'a, Sha256>;

pub fn event_merkle_tree_from_bytes(bytes: &[u8; 90368]) -> &EventMerkleTree {
    // SAFETY: We make sure that the size of the byte slice is equal to
    // the size of `EventMerkleTree`.
    // The only reason why we are doing this is that Anchor is struggling with
    // generating IDL when `ConcurrentMerkleTree` with generics is used
    // directly as a field.
    unsafe {
        let ptr = bytes.as_ptr() as *const EventMerkleTree;
        &*ptr
    }
}

pub fn event_merkle_tree_from_bytes_mut(bytes: &mut [u8; 90368]) -> &mut EventMerkleTree {
    // SAFETY: We make sure that the size of the byte slice is equal to
    // the size of `EventMerkleTree`.
    // The only reason why we are doing this is that Anchor is struggling with
    // generating IDL when `ConcurrentMerkleTree` with generics is used
    // directly as a field.
    unsafe {
        let ptr = bytes.as_ptr() as *mut EventMerkleTree;
        &mut *ptr
    }
}

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

        state_merkle_tree_from_bytes_mut(&mut self.state_merkle_tree)
            .init()
            .map_err(ProgramError::from)?;
        event_merkle_tree_from_bytes_mut(&mut self.event_merkle_tree)
            .init()
            .map_err(ProgramError::from)?;

        Ok(())
    }
}
