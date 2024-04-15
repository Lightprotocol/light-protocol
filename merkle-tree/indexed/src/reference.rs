use std::marker::PhantomData;

use light_bounded_vec::{BoundedVec, BoundedVecError};
use light_concurrent_merkle_tree::light_hasher::{errors::HasherError, Hasher};
use light_merkle_tree_reference::{MerkleTree, ReferenceMerkleTreeError};
use light_utils::bigint::bigint_to_be_bytes_array;
use num_bigint::BigUint;
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned};
use thiserror::Error;

use crate::{
    array::{IndexedArray, IndexedElement},
    errors::IndexedMerkleTreeError,
};

#[derive(Debug, Error)]
pub enum IndexedReferenceMerkleTreeError {
    #[error(transparent)]
    Indexed(#[from] IndexedMerkleTreeError),
    #[error(transparent)]
    Reference(#[from] ReferenceMerkleTreeError),
    #[error(transparent)]
    Hasher(#[from] HasherError),
}

#[derive(Debug)]
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

    // TODO: rename input values
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

    // TODO: add append with new value, so that we don't need to compute the lowlevel values manually
    pub fn append<const T: usize>(
        &mut self,
        value: &BigUint,
        indexed_array: &mut IndexedArray<H, I, T>,
    ) -> Result<(), IndexedReferenceMerkleTreeError> {
        let nullifier_bundle = indexed_array.append(value).unwrap();
        self.update(
            &nullifier_bundle.new_low_element,
            &nullifier_bundle.new_element,
            &nullifier_bundle.new_element_next_value,
        )?;

        Ok(())
    }

    pub fn get_non_inclusion_proof<const T: usize>(
        &self,
        value: &BigUint,
        indexed_array: &IndexedArray<H, I, T>,
    ) -> Result<NonInclusionProof, IndexedReferenceMerkleTreeError> {
        let (low_element, _next_value) = indexed_array.find_low_element(value)?;
        let merkle_proof = self
            .get_proof_of_leaf(usize::from(low_element.index), true)
            .unwrap();
        let higher_range_value = indexed_array
            .get(low_element.next_index())
            .unwrap()
            .value
            .clone();
        Ok(NonInclusionProof {
            root: self.root(),
            value: bigint_to_be_bytes_array::<32>(value).unwrap(),
            leaf_lower_range_value: bigint_to_be_bytes_array::<32>(&low_element.value).unwrap(),
            leaf_higher_range_value: bigint_to_be_bytes_array::<32>(&higher_range_value).unwrap(),
            leaf_index: low_element.index.into(),
            next_index: low_element.next_index(),
            merkle_proof,
        })
    }

    pub fn verify_non_inclusion_proof(
        &self,
        proof: &NonInclusionProof,
    ) -> Result<(), IndexedReferenceMerkleTreeError> {
        let array_element = IndexedElement::<usize> {
            value: BigUint::from_bytes_be(&proof.value),
            index: proof.leaf_index,
            next_index: proof.next_index,
        };
        let leaf_hash =
            array_element.hash::<H>(&BigUint::from_bytes_be(&proof.leaf_higher_range_value))?;
        self.merkle_tree
            .verify(&leaf_hash, &proof.merkle_proof, proof.leaf_index)
            .unwrap();
        Ok(())
    }
}

// TODO: check why next_index is usize while index is I
/// We prove non-inclusion by:
/// 1. Showing that value is greater than leaf_lower_range_value and less than leaf_higher_range_value
/// 2. Showing that the leaf_hash H(leaf_lower_range_value, leaf_next_index, leaf_higher_value) is included in the root (Merkle tree)
pub struct NonInclusionProof<'a> {
    pub root: [u8; 32],
    pub value: [u8; 32],
    pub leaf_lower_range_value: [u8; 32],
    pub leaf_higher_range_value: [u8; 32],
    pub leaf_index: usize,
    pub next_index: usize,
    pub merkle_proof: BoundedVec<'a, [u8; 32]>,
}
