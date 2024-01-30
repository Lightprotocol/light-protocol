use ark_ff::BigInteger256;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{array::IndexingArray, IndexedMerkleTree};

pub const QUEUE_ELEMENTS: usize = 2800;

pub const MERKLE_TREE_HEIGHT: usize = 22;
pub const MERKLE_TREE_CHANGELOG: usize = 2800;
pub const MERKLE_TREE_ROOTS: usize = 2800;

pub type AddressQueue = IndexingArray<Poseidon, BigInteger256, QUEUE_ELEMENTS>;

pub fn address_queue_from_bytes(bytes: &[u8; 112008]) -> &AddressQueue {
    // SAFETY: We make sure that the size of the byte slice is equal to
    // the size of `StateMerkleTree`.
    // The only reason why we are doing this is that Anchor is struggling with
    // generating IDL when `ConcurrentMerkleTree` with generics is used
    // directly as a field.
    unsafe {
        let ptr = bytes.as_ptr() as *const AddressQueue;
        &*ptr
    }
}

pub fn address_queue_from_bytes_mut(bytes: &mut [u8; 112008]) -> &mut AddressQueue {
    // SAFETY: We make sure that the size of the byte slice is equal to
    // the size of `StateMerkleTree`.
    // The only reason why we are doing this is that Anchor is struggling with
    // generating IDL when `ConcurrentMerkleTree` with generics is used
    // directly as a field.
    unsafe {
        let ptr = bytes.as_ptr() as *mut AddressQueue;
        &mut *ptr
    }
}

pub type AddressMerkleTree = IndexedMerkleTree<
    Poseidon,
    BigInteger256,
    MERKLE_TREE_HEIGHT,
    MERKLE_TREE_CHANGELOG,
    MERKLE_TREE_ROOTS,
>;

pub fn address_merkle_tree_from_bytes(bytes: &[u8; 2173568]) -> &AddressMerkleTree {
    // SAFETY: We make sure that the size of the byte slice is equal to
    // the size of `StateMerkleTree`.
    // The only reason why we are doing this is that Anchor is struggling with
    // generating IDL when `ConcurrentMerkleTree` with generics is used
    // directly as a field.
    unsafe {
        let ptr = bytes.as_ptr() as *const AddressMerkleTree;
        &*ptr
    }
}

pub fn address_merkle_tree_from_bytes_mut(bytes: &mut [u8; 2173568]) -> &mut AddressMerkleTree {
    // SAFETY: We make sure that the size of the byte slice is equal to
    // the size of `StateMerkleTree`.
    // The only reason why we are doing this is that Anchor is struggling with
    // generating IDL when `ConcurrentMerkleTree` with generics is used
    // directly as a field.
    unsafe {
        let ptr = bytes.as_ptr() as *mut AddressMerkleTree;
        &mut *ptr
    }
}
