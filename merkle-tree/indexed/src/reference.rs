use std::marker::PhantomData;

use ark_ff::BigInteger;
use light_hasher::{errors::HasherError, Hasher};
use light_merkle_tree_reference::MerkleTree;

use crate::array::IndexingElement;

#[repr(C)]
pub struct IndexedMerkleTree<H, B, const HEIGHT: usize, const MAX_ROOTS: usize>
where
    H: Hasher,
    B: BigInteger,
{
    pub merkle_tree: MerkleTree<H, HEIGHT, MAX_ROOTS>,
    _bigint: PhantomData<B>,
}

impl<H, B, const HEIGHT: usize, const MAX_ROOTS: usize> IndexedMerkleTree<H, B, HEIGHT, MAX_ROOTS>
where
    H: Hasher,
    B: BigInteger,
{
    pub fn new() -> Result<Self, HasherError> {
        let mut merkle_tree = MerkleTree::new()?;

        // Append the first low leaf, which has value 0 and does not point
        // to any other leaf yet.
        // This low leaf is going to be updated during the first `update`
        // operation.
        merkle_tree.update(&H::zero_indexed_leaf(), 0)?;

        Ok(Self {
            merkle_tree,
            _bigint: PhantomData,
        })
    }

    pub fn get_proof_of_leaf(&self, index: usize) -> [[u8; 32]; HEIGHT] {
        self.merkle_tree.get_proof_of_leaf(index)
    }

    pub fn root(&self) -> Option<[u8; 32]> {
        self.merkle_tree.root()
    }

    pub fn update(
        &mut self,
        new_low_element: &IndexingElement<B>,
        new_element: &IndexingElement<B>,
        new_element_next_value: &B,
    ) -> Result<(), HasherError> {
        // Update the low element.
        let new_low_leaf = new_low_element.hash::<H>(&new_element.value)?;
        self.merkle_tree
            .update(&new_low_leaf, new_low_element.index as usize)?;

        // Append the new element.
        let new_leaf = new_element.hash::<H>(new_element_next_value)?;
        self.merkle_tree
            .update(&new_leaf, new_element.index as usize)?;

        Ok(())
    }
}
