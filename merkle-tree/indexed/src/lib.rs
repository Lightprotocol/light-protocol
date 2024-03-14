use std::marker::PhantomData;

use array::IndexedElement;
use light_bounded_vec::BoundedVec;
use light_concurrent_merkle_tree::{
    errors::ConcurrentMerkleTreeError, light_hasher::Hasher, ConcurrentMerkleTree,
};
use num_bigint::BigUint;
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned};

pub mod array;
pub mod errors;
pub mod reference;

use crate::errors::IndexedMerkleTreeError;

#[repr(C)]
pub struct IndexedMerkleTree<'a, H, I, const HEIGHT: usize>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>,
{
    pub merkle_tree: ConcurrentMerkleTree<'a, H, HEIGHT>,
    _index: PhantomData<I>,
}

pub type IndexedMerkleTree22<'a, H, I> = IndexedMerkleTree<'a, H, I, 22>;
pub type IndexedMerkleTree26<'a, H, I> = IndexedMerkleTree<'a, H, I, 26>;
pub type IndexedMerkleTree32<'a, H, I> = IndexedMerkleTree<'a, H, I, 32>;
pub type IndexedMerkleTree40<'a, H, I> = IndexedMerkleTree<'a, H, I, 40>;

impl<'a, H, I, const HEIGHT: usize> IndexedMerkleTree<'a, H, I, HEIGHT>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>,
{
    pub fn new(
        height: usize,
        changelog_size: usize,
        roots_size: usize,
        canopy_depth: usize,
    ) -> Self {
        let merkle_tree = ConcurrentMerkleTree::<H, HEIGHT>::new(
            height,
            changelog_size,
            roots_size,
            canopy_depth,
        );
        Self {
            merkle_tree,
            _index: PhantomData,
        }
    }

    /// Casts byte slices into `ConcurrentMerkleTree`.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. Ensuring the size and alignment of the byte
    /// slices is the caller's responsibility.
    pub unsafe fn from_bytes<'b>(
        bytes_struct: &'b [u8],
        bytes_filled_subtrees: &'b [u8],
        bytes_changelog: &'b [u8],
        bytes_roots: &'b [u8],
        bytes_canopy: &'b [u8],
    ) -> Result<&'b Self, ConcurrentMerkleTreeError> {
        let merkle_tree = ConcurrentMerkleTree::<H, HEIGHT>::from_bytes(
            bytes_struct,
            bytes_filled_subtrees,
            bytes_changelog,
            bytes_roots,
            bytes_canopy,
        )?;

        Ok(&*(merkle_tree as *const ConcurrentMerkleTree<H, HEIGHT> as *const Self))
    }

    /// Casts byte slices into `ConcurrentMerkleTree`.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. Ensuring the size and alignment of the byte
    /// slices is the caller's responsibility.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn from_bytes_init(
        bytes_struct: &'a mut [u8],
        bytes_filled_subtrees: &'a mut [u8],
        bytes_changelog: &'a mut [u8],
        bytes_roots: &'a mut [u8],
        bytes_canopy: &'a mut [u8],
        height: usize,
        changelog_size: usize,
        roots_size: usize,
        canopy_depth: usize,
    ) -> Result<&'a mut Self, ConcurrentMerkleTreeError> {
        let merkle_tree = ConcurrentMerkleTree::<H, HEIGHT>::from_bytes_init(
            bytes_struct,
            bytes_filled_subtrees,
            bytes_changelog,
            bytes_roots,
            bytes_canopy,
            height,
            changelog_size,
            roots_size,
            canopy_depth,
        )?;

        Ok(&mut *(merkle_tree as *mut ConcurrentMerkleTree<H, HEIGHT> as *mut Self))
    }

    /// Casts byte slices into `ConcurrentMerkleTree`.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. Ensuring the size and alignment of the byte
    /// slices is the caller's responsibility.
    pub unsafe fn from_bytes_mut<'b>(
        bytes_struct: &'b mut [u8],
        bytes_filled_subtrees: &'b mut [u8],
        bytes_changelog: &'b mut [u8],
        bytes_roots: &'b mut [u8],
        bytes_canopy: &'b mut [u8],
    ) -> Result<&'b mut Self, ConcurrentMerkleTreeError> {
        let merkle_tree = ConcurrentMerkleTree::<H, HEIGHT>::from_bytes_mut(
            bytes_struct,
            bytes_filled_subtrees,
            bytes_changelog,
            bytes_roots,
            bytes_canopy,
        )?;

        Ok(&mut *(merkle_tree as *mut ConcurrentMerkleTree<H, HEIGHT> as *mut Self))
    }

    pub fn init(&mut self) -> Result<(), IndexedMerkleTreeError> {
        self.merkle_tree.init()?;

        // Append the first low leaf, which has value 0 and does not point
        // to any other leaf yet.
        // This low leaf is going to be updated during the first `update`
        // operation.
        self.merkle_tree.append(&H::zero_indexed_leaf())?;

        Ok(())
    }

    pub fn changelog_index(&self) -> usize {
        self.merkle_tree.changelog_index()
    }

    pub fn root_index(&self) -> usize {
        self.merkle_tree.root_index()
    }

    pub fn root(&self) -> Result<[u8; 32], IndexedMerkleTreeError> {
        let root = self.merkle_tree.root()?;
        Ok(root)
    }

    /// Checks whether the given Merkle `proof` for the given `node` (with index
    /// `i`) is valid. The proof is valid when computing parent node hashes using
    /// the whole path of the proof gives the same result as the given `root`.
    pub fn validate_proof(
        &self,
        leaf: &[u8; 32],
        leaf_index: usize,
        proof: &BoundedVec<[u8; 32]>,
    ) -> Result<(), IndexedMerkleTreeError> {
        self.merkle_tree.validate_proof(leaf, leaf_index, proof)?;
        Ok(())
    }

    pub fn update(
        &mut self,
        changelog_index: usize,
        new_element: IndexedElement<I>,
        new_element_next_value: &BigUint,
        low_element: IndexedElement<I>,
        low_element_next_value: &BigUint,
        low_leaf_proof: &mut BoundedVec<[u8; 32]>,
    ) -> Result<(), IndexedMerkleTreeError> {
        // Check that the value of `new_element` belongs to the range
        // of `old_low_element`.
        if low_element.next_index == I::zero() {
            // In this case, the `old_low_element` is the greatest element.
            // The value of `new_element` needs to be greater than the value of
            // `old_low_element` (and therefore, be the greatest).
            if new_element.value <= low_element.value {
                return Err(IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement);
            }
        } else {
            // The value of `new_element` needs to be greater than the value of
            // `old_low_element` (and therefore, be the greatest).
            if new_element.value <= low_element.value {
                return Err(IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement);
            }
            // The value of `new_element` needs to be lower than the value of
            // next element pointed by `old_low_element`.
            if new_element.value >= *low_element_next_value {
                return Err(IndexedMerkleTreeError::NewElementGreaterOrEqualToNextElement);
            }
        }

        // Instantiate `new_low_element` - the low element with updated values.
        let new_low_element = IndexedElement {
            index: low_element.index,
            value: low_element.value.clone(),
            next_index: new_element.index,
        };

        // Update low element. If the `old_low_element` does not belong to the
        // tree, validating the proof is going to fail.
        let old_low_leaf = low_element.hash::<H>(low_element_next_value).map_err(|e| {
            println!("error 1");
            e
        })?;
        let new_low_leaf = new_low_element.hash::<H>(&new_element.value).map_err(|e| {
            println!("error 2");
            e
        })?;
        self.merkle_tree.update(
            changelog_index,
            &old_low_leaf,
            &new_low_leaf,
            low_element.index.into(),
            low_leaf_proof,
        )?;

        // Append new element.
        let new_leaf = new_element.hash::<H>(new_element_next_value).map_err(|e| {
            println!("error 3");
            e
        })?;
        self.merkle_tree.append(&new_leaf)?;

        Ok(())
    }
}
