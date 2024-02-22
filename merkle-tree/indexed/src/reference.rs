use std::marker::PhantomData;

use ark_ff::BigInteger;
use light_concurrent_merkle_tree::light_hasher::Hasher;
use light_merkle_tree_reference::MerkleTree;
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned};

use crate::{array::IndexingElement, errors::IndexedMerkleTreeError};

#[repr(C)]
pub struct IndexedMerkleTree<H, I, B, const HEIGHT: usize, const MAX_ROOTS: usize>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    B: BigInteger,
{
    pub merkle_tree: MerkleTree<H, HEIGHT, MAX_ROOTS>,
    _bigint: PhantomData<B>,
    _index: PhantomData<I>,
}

impl<H, I, B, const HEIGHT: usize, const MAX_ROOTS: usize>
    IndexedMerkleTree<H, I, B, HEIGHT, MAX_ROOTS>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    B: BigInteger,
    usize: From<I>,
{
    pub fn new() -> Result<Self, IndexedMerkleTreeError> {
        let mut merkle_tree = MerkleTree::new()?;

        // Append the first low leaf, which has value 0 and does not point
        // to any other leaf yet.
        // This low leaf is going to be updated during the first `update`
        // operation.
        merkle_tree.update(&H::zero_indexed_leaf(), 0)?;

        Ok(Self {
            merkle_tree,
            _bigint: PhantomData,
            _index: PhantomData,
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
        new_low_element: &IndexingElement<I, B>,
        new_element: &IndexingElement<I, B>,
        new_element_next_value: &B,
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
