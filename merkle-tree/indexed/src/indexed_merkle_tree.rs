use light_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree;
use light_hasher::Hasher;

use crate::{errors::IndexedMerkleTreeError, indexing_array::IndexingArray};

#[repr(C)]
pub struct IndexedMerkleTree<
    H,
    const MAX_HEIGHT: usize,
    const MAX_ROOTS: usize,
    const MAX_LEAVES: usize,
> where
    H: Hasher,
{
    merkle_tree: ConcurrentMerkleTree<H, MAX_HEIGHT, MAX_ROOTS>,
    indexing_array: IndexingArray<MAX_LEAVES>,
}

impl<H, const MAX_HEIGHT: usize, const MAX_ROOTS: usize, const MAX_LEAVES: usize>
    IndexedMerkleTree<H, MAX_HEIGHT, MAX_ROOTS, MAX_LEAVES>
where
    H: Hasher,
{
    fn default() -> Self {
        Self {
            merkle_tree: ConcurrentMerkleTree::default(),
            indexing_array: IndexingArray::default(),
        }
    }
}

impl<H, const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize, const MAX_ELEMENTS: usize>
    IndexedMerkleTree<H, MAX_DEPTH, MAX_BUFFER_SIZE, MAX_ELEMENTS>
where
    H: Hasher,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, value: [u8; 32]) -> Result<(), IndexedMerkleTreeError> {
        let (low_element_index, new_node_index) = self.indexing_array.append(value)?;

        let low_element_hash = H::hashv(&[
            &self.indexing_array.nodes[low_element_index].value, // value
            &new_node_index.to_le_bytes(),                       // next index
            &self.indexing_array.nodes[new_node_index].value,    // next value
        ])
        .unwrap();

        self.merkle_tree
            .replace_leaf(
                [0u8; 32], // root
                [0u8; 32], // old leaf
                low_element_hash,
                low_element_index,
                &[], // proof
            )
            .unwrap(); // TODO: Handle error.

        let next_node_index = self.indexing_array.nodes[new_node_index].next_index;
        let new_node_hash = H::hashv(&[
            &self.indexing_array.nodes[new_node_index].value, // value
            &self.indexing_array.nodes[new_node_index]
                .next_index
                .to_be_bytes(), // next index
            &self.indexing_array.nodes[next_node_index].value, // next value
        ])
        .unwrap();

        self.merkle_tree.append(new_node_hash).unwrap();

        Ok(())
    }
}
