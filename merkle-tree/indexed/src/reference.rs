use std::marker::PhantomData;

use light_bounded_vec::{BoundedVec, BoundedVecError};
use light_concurrent_merkle_tree::light_hasher::Hasher;
use light_merkle_tree_reference::MerkleTree;
use num_bigint::BigUint;
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned};

use crate::{array::IndexedElement, errors::IndexedMerkleTreeError};

#[repr(C)]
pub struct IndexedMerkleTree<H, I>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
{
    pub merkle_tree: MerkleTree<H>,
    _index: PhantomData<I>,
}

impl<H, I> IndexedMerkleTree<H, I>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>,
{
    pub fn new(
        height: usize,
        roots_size: usize,
        canopy_depth: usize,
    ) -> Result<Self, IndexedMerkleTreeError> {
        let mut merkle_tree = MerkleTree::new(height, roots_size, canopy_depth)?;

        // Append the first low leaf, which has value 0 and does not point
        // to any other leaf yet.
        // This low leaf is going to be updated during the first `update`
        // operation.
        merkle_tree.update(&H::zero_indexed_leaf(), 0)?;

        Ok(Self {
            merkle_tree,
            _index: PhantomData,
        })
    }

    pub fn get_proof_of_leaf(
        &self,
        index: usize,
        full: bool,
    ) -> Result<BoundedVec<[u8; 32]>, BoundedVecError> {
        self.merkle_tree.get_proof_of_leaf(index, full)
    }

    pub fn root(&self) -> Option<[u8; 32]> {
        self.merkle_tree.root()
    }

    pub fn update(
        &mut self,
        new_low_element: &IndexedElement<I>,
        new_element: &IndexedElement<I>,
        new_element_next_value: &BigUint,
    ) -> Result<(), IndexedMerkleTreeError> {
        // Update the low element.
        let new_low_leaf = new_low_element.hash::<H>(&new_element.value)?;
        self.merkle_tree
            .update(&new_low_leaf, usize::from(new_low_element.index))?;

        // Append the new element.
        let new_leaf = new_element.hash::<H>(new_element_next_value)?;
        self.merkle_tree
            .update(&new_leaf, usize::from(new_element.index))?;

        Ok(())
    }
}
