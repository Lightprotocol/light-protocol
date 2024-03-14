use light_concurrent_merkle_tree::ConcurrentMerkleTree26;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::IndexedMerkleTree22;

/// Size of the address space queue.
pub const QUEUE_ELEMENTS: usize = 2800;

pub type StateMerkleTree<'a> = ConcurrentMerkleTree26<'a, Poseidon>;

pub type AddressMerkleTree<'a> = IndexedMerkleTree22<'a, Poseidon, usize>;
