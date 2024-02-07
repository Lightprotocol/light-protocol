use light_concurrent_merkle_tree::ConcurrentMerkleTree;
use light_hasher::{Poseidon, Sha256};

pub const MERKLE_TREE_HEIGHT: usize = 22;
pub const MERKLE_TREE_CHANGELOG: usize = 0;
pub const MERKLE_TREE_ROOTS: usize = 2800;

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

pub type EventMerkleTree =
    ConcurrentMerkleTree<Sha256, MERKLE_TREE_HEIGHT, MERKLE_TREE_CHANGELOG, MERKLE_TREE_ROOTS>;

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
