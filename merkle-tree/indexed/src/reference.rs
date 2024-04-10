use std::marker::PhantomData;

use light_bounded_vec::{BoundedVec, BoundedVecError};
use light_concurrent_merkle_tree::light_hasher::{errors::HasherError, Hasher};
use light_merkle_tree_reference::{MerkleTree, ReferenceMerkleTreeError};
use num_bigint::BigUint;
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned};
use thiserror::Error;

use crate::{array::IndexedElement, errors::IndexedMerkleTreeError};

#[derive(Debug, Error)]
pub enum IndexedReferenceMerkleTreeError {
    #[error(transparent)]
    Indexed(#[from] IndexedMerkleTreeError),
    #[error(transparent)]
    Reference(#[from] ReferenceMerkleTreeError),
    #[error(transparent)]
    Hasher(#[from] HasherError),
}

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
        canopy_depth: usize,
    ) -> Result<Self, IndexedReferenceMerkleTreeError> {
        let mut merkle_tree = MerkleTree::new(height, canopy_depth);

        // Append the first low leaf, which has value 0 and does not point
        // to any other leaf yet.
        // This low leaf is going to be updated during the first `update`
        // operation.
        merkle_tree.append(&H::zero_indexed_leaf())?;

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

    pub fn root(&self) -> [u8; 32] {
        self.merkle_tree.root()
    }

    pub fn update(
        &mut self,
        new_low_element: &IndexedElement<I>,
        new_element: &IndexedElement<I>,
        new_element_next_value: &BigUint,
    ) -> Result<(), IndexedReferenceMerkleTreeError> {
        // Update the low element.
        let new_low_leaf = new_low_element.hash::<H>(&new_element.value)?;
        self.merkle_tree
            .update(&new_low_leaf, usize::from(new_low_element.index))?;

        // Append the new element.
        let new_leaf = new_element.hash::<H>(new_element_next_value)?;
        self.merkle_tree.append(&new_leaf)?;

        Ok(())
    }
}
