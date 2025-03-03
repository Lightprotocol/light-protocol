use std::{marker::PhantomData, ops::DerefMut};

use light_compressed_account::bigint::bigint_to_be_bytes_array;
use light_hasher::{Hasher, HasherError};
use light_indexed_array::{
    array::{IndexedArray, IndexedElement},
    errors::IndexedArrayError,
    HIGHEST_ADDRESS_PLUS_ONE,
};
use num_bigint::BigUint;
use num_traits::{CheckedAdd, CheckedSub, Num, ToBytes, Unsigned};
use thiserror::Error;

use crate::{MerkleTree, ReferenceMerkleTreeError};

#[derive(Debug, Error)]
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

        let mut indexed_array = IndexedArray::<H, I>::default();
        let init_value = BigUint::from_str_radix(HIGHEST_ADDRESS_PLUS_ONE, 10).unwrap();
        let nullifier_bundle = indexed_array.append(&init_value)?;
        // let new_low_leaf = nullifier_bundle
        //     .new_low_element
        //     .hash::<H>(&nullifier_bundle.new_element.value)?;
        let new_leaf = nullifier_bundle
            .new_element
            .hash::<H>(&nullifier_bundle.new_element_next_value)?;
        merkle_tree.append(&new_leaf)?;
        assert_eq!(merkle_tree.leaf(0), new_leaf);

        Ok(Self {
            merkle_tree,
            _index: PhantomData,
        })
    }

    // /// Initializes the reference indexed merkle tree on par with the
    // /// on-chain indexed concurrent merkle tree.
    // /// Inserts the ranges 0 - BN254 Field Size - 1 into the tree.
    // pub fn init(&mut self) -> Result<(), ReferenceMerkleTreeError> {
    //     let mut indexed_array = IndexedArray::<H, I>::default();
    //     let init_value = BigUint::from_str_radix(HIGHEST_ADDRESS_PLUS_ONE, 10).unwrap();
    //     let nullifier_bundle = indexed_array.append(&init_value)?;
    //     let new_low_leaf = nullifier_bundle
    //         .new_low_element
    //         .hash::<H>(&nullifier_bundle.new_element.value)?;

    //     self.merkle_tree.update(&new_low_leaf, 0)?;
    //     let new_leaf = nullifier_bundle
    //         .new_element
    //         .hash::<H>(&nullifier_bundle.new_element_next_value)?;
    //     self.merkle_tree.append(&new_leaf)?;
    //     Ok(())
    // }

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
        self.merkle_tree
            .update(&new_low_leaf, usize::from(new_low_element.index))?;
        println!("reference updated root {:?}", self.merkle_tree.root());
        // Append the new element.
        let new_leaf = new_element.hash::<H>(new_element_next_value)?;
        println!("reference update new leaf hash {:?}", new_leaf);
        self.merkle_tree.append(&new_leaf)?;
        println!("reference appended root {:?}", self.merkle_tree.root());

        Ok(())
    }

    // TODO: add append with new value, so that we don't need to compute the lowlevel values manually
    pub fn append(
        &mut self,
        value: &BigUint,
        indexed_array: &mut IndexedArray<H, I>,
    ) -> Result<(), IndexedReferenceMerkleTreeError> {
        println!("appending {:?}", value);
        let nullifier_bundle = indexed_array.append(value).unwrap();
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
        indexed_array: &IndexedArray<H, I>,
    ) -> Result<NonInclusionProof, IndexedReferenceMerkleTreeError> {
        let (low_element, _next_value) = indexed_array.find_low_element_for_nonexistent(value)?;
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
            .verify(&leaf_hash, &proof.merkle_proof, proof.leaf_index)
            .unwrap();
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
