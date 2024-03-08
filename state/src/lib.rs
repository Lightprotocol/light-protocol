use ark_ff::BigInteger256;
use light_concurrent_merkle_tree::ConcurrentMerkleTree26;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{array::IndexingArray, IndexedMerkleTree22};

/// Size of the address space queue.
pub const QUEUE_ELEMENTS: usize = 2800;

pub type StateMerkleTree<'a> = ConcurrentMerkleTree26<'a, Poseidon>;

pub type AddressQueue = IndexingArray<Poseidon, u16, BigInteger256, QUEUE_ELEMENTS>;

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

pub type AddressMerkleTree<'a> = IndexedMerkleTree22<'a, Poseidon, usize, BigInteger256>;
