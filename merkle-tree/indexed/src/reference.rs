use light_hasher::{errors::HasherError, Hasher};
use light_merkle_tree_reference::MerkleTree;

use crate::array::IndexingElement;

#[repr(C)]
pub struct IndexedMerkleTree<H, const HEIGHT: usize, const MAX_ROOTS: usize>
where
    H: Hasher,
{
    pub merkle_tree: MerkleTree<H, HEIGHT, MAX_ROOTS>,
}

impl<H, const HEIGHT: usize, const MAX_ROOTS: usize> IndexedMerkleTree<H, HEIGHT, MAX_ROOTS>
where
    H: Hasher,
{
    pub fn new() -> Result<Self, HasherError> {
        let mut merkle_tree = MerkleTree::new()?;

        // Append the first low leaf, which has value 0 and does not point
        // to any other leaf yet.
        // This low leaf is going to be updated during the first `update`
        // operation.
        merkle_tree.update(&H::zero_indexed_leaf(), 0)?;

        Ok(Self { merkle_tree })
    }

    pub fn get_proof_of_leaf(&self, index: usize) -> [[u8; 32]; HEIGHT] {
        self.merkle_tree.get_proof_of_leaf(index)
    }

    pub fn node(&self, idx: usize) -> [u8; 32] {
        self.merkle_tree.node(idx)
    }

    pub fn root(&self) -> Option<[u8; 32]> {
        self.merkle_tree.root()
    }

    pub fn update(
        &mut self,
        new_low_element: IndexingElement,
        low_leaf_index: usize,
        new_element: IndexingElement,
        leaf_index: usize,
    ) -> Result<(), HasherError> {
        // Update the low element.
        let new_low_leaf = new_low_element.hash::<H>()?;
        self.merkle_tree.update(&new_low_leaf, low_leaf_index)?;

        // Append the new element.
        let new_leaf = new_element.hash::<H>()?;
        self.merkle_tree.update(&new_leaf, leaf_index)?;

        Ok(())
    }
}
