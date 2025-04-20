use std::{fmt::Debug, marker::PhantomData};

use light_hasher::{bigint::bigint_to_be_bytes_array, Hasher, HasherError};
use light_indexed_array::{
    array::{IndexedArray, IndexedElement},
    errors::IndexedArrayError,
    HIGHEST_ADDRESS_PLUS_ONE,
};
use num_bigint::BigUint;
use num_traits::{CheckedAdd, CheckedSub, Num, ToBytes, Unsigned, Zero};
use thiserror::Error;

use crate::{MerkleTree, ReferenceMerkleTreeError};

#[derive(Debug, Error, PartialEq)]
pub enum IndexedReferenceMerkleTreeError {
    #[error("NonInclusionProofFailedLowerBoundViolated")]
    NonInclusionProofFailedLowerBoundViolated,
    #[error("NonInclusionProofFailedHigherBoundViolated")]
    NonInclusionProofFailedHigherBoundViolated,
    #[error(transparent)]
    Indexed(#[from] IndexedArrayError),
    #[error(transparent)]
    Reference(#[from] ReferenceMerkleTreeError),
    #[error(transparent)]
    Hasher(#[from] HasherError),
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct IndexedMerkleTree<H, I>
where
    H: Hasher,
    I: CheckedAdd
        + CheckedSub
        + Copy
        + Clone
        + PartialOrd
        + ToBytes
        + TryFrom<usize>
        + Unsigned
        + Debug
        + Into<usize>,
{
    pub merkle_tree: MerkleTree<H>,
    pub indexed_array: IndexedArray<H, I>,
    _index: PhantomData<I>,
}

impl<H, I> IndexedMerkleTree<H, I>
where
    H: Hasher,
    I: CheckedAdd
        + CheckedSub
        + Copy
        + Clone
        + PartialOrd
        + ToBytes
        + TryFrom<usize>
        + Unsigned
        + Debug
        + Into<usize>,
{
    pub fn new(
        height: usize,
        canopy_depth: usize,
    ) -> Result<Self, IndexedReferenceMerkleTreeError> {
        let mut merkle_tree = MerkleTree::new(height, canopy_depth);

        let init_next_value = BigUint::from_str_radix(HIGHEST_ADDRESS_PLUS_ONE, 10).unwrap();
        let indexed_array = IndexedArray::<H, I>::new(BigUint::zero(), init_next_value.clone());
        let new_leaf = indexed_array
            .get(0)
            .unwrap()
            .hash::<H>(&init_next_value)
            .unwrap();
        merkle_tree.append(&new_leaf)?;
        assert_eq!(merkle_tree.leaf(0), new_leaf);
        Ok(Self {
            merkle_tree,
            indexed_array,
            _index: PhantomData,
        })
    }

    pub fn get_path_of_leaf(
        &self,
        index: usize,
        full: bool,
    ) -> Result<Vec<[u8; 32]>, ReferenceMerkleTreeError> {
        self.merkle_tree.get_path_of_leaf(index, full)
    }

    pub fn get_proof_of_leaf(
        &self,
        index: usize,
        full: bool,
    ) -> Result<Vec<[u8; 32]>, ReferenceMerkleTreeError> {
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
        println!("reference update new low leaf hash {:?}", new_low_leaf);
        self.merkle_tree.update(
            &new_low_leaf,
            usize::try_from(new_low_element.index).unwrap(),
        )?;
        println!("reference updated root {:?}", self.merkle_tree.root());
        // Append the new element.
        let new_leaf = new_element.hash::<H>(new_element_next_value)?;
        println!("reference update new leaf hash {:?}", new_leaf);
        self.merkle_tree.append(&new_leaf)?;
        println!("reference appended root {:?}", self.merkle_tree.root());

        Ok(())
    }

    // TODO: add append with new value, so that we don't need to compute the lowlevel values manually
    pub fn append(&mut self, value: &BigUint) -> Result<(), IndexedReferenceMerkleTreeError> {
        println!("\n\nappending {:?}", value);
        let nullifier_bundle = self.indexed_array.append(value)?;
        println!("\n\nnullifier_bundle {:?}", nullifier_bundle);
        self.update(
            &nullifier_bundle.new_low_element,
            &nullifier_bundle.new_element,
            &nullifier_bundle.new_element_next_value,
        )?;

        Ok(())
    }

    pub fn get_non_inclusion_proof(
        &self,
        value: &BigUint,
    ) -> Result<NonInclusionProof, IndexedReferenceMerkleTreeError> {
        let (low_element, _next_value) =
            self.indexed_array.find_low_element_for_nonexistent(value)?;
        let merkle_proof =
            self.get_proof_of_leaf(usize::try_from(low_element.index).unwrap(), true)?;
        let higher_range_value = if low_element.next_index() == 0 {
            self.indexed_array.highest_value.clone()
        } else {
            self.indexed_array
                .get(low_element.next_index())
                .unwrap()
                .value
                .clone()
        };
        let non_inclusion_proof = NonInclusionProof {
            root: self.root(),
            value: bigint_to_be_bytes_array::<32>(value).unwrap(),
            leaf_lower_range_value: bigint_to_be_bytes_array::<32>(&low_element.value).unwrap(),
            leaf_higher_range_value: bigint_to_be_bytes_array::<32>(&higher_range_value).unwrap(),
            leaf_index: low_element.index.into(),
            next_index: low_element.next_index(),
            merkle_proof,
        };
        Ok(non_inclusion_proof)
    }

    pub fn verify_non_inclusion_proof(
        &self,
        proof: &NonInclusionProof,
    ) -> Result<(), IndexedReferenceMerkleTreeError> {
        let value_big_int = BigUint::from_bytes_be(&proof.value);
        let lower_end_value = BigUint::from_bytes_be(&proof.leaf_lower_range_value);
        if lower_end_value >= value_big_int {
            return Err(IndexedReferenceMerkleTreeError::NonInclusionProofFailedLowerBoundViolated);
        }
        let higher_end_value = BigUint::from_bytes_be(&proof.leaf_higher_range_value);
        if higher_end_value <= value_big_int {
            return Err(
                IndexedReferenceMerkleTreeError::NonInclusionProofFailedHigherBoundViolated,
            );
        }

        let array_element = IndexedElement::<usize> {
            value: lower_end_value,
            index: proof.leaf_index,
            next_index: proof.next_index,
        };
        let leaf_hash = array_element.hash::<H>(&higher_end_value)?;
        self.merkle_tree
            .verify(&leaf_hash, &proof.merkle_proof, proof.leaf_index)?;
        Ok(())
    }
}

// TODO: check why next_index is usize while index is I
/// We prove non-inclusion by:
/// 1. Showing that value is greater than leaf_lower_range_value and less than leaf_higher_range_value
/// 2. Showing that the leaf_hash H(leaf_lower_range_value, leaf_next_index, leaf_higher_value) is included in the root (Merkle tree)
#[derive(Debug)]
pub struct NonInclusionProof {
    pub root: [u8; 32],
    pub value: [u8; 32],
    pub leaf_lower_range_value: [u8; 32],
    pub leaf_higher_range_value: [u8; 32],
    pub leaf_index: usize,
    pub next_index: usize,
    pub merkle_proof: Vec<[u8; 32]>,
}
