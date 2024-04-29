use std::marker::PhantomData;

use array::{IndexedArray, IndexedElement};
use light_bounded_vec::{BoundedVec, CyclicBoundedVec, Pod};
use light_concurrent_merkle_tree::{
    errors::ConcurrentMerkleTreeError, light_hasher::Hasher, ConcurrentMerkleTree,
};
use light_utils::bigint::bigint_to_be_bytes_array;
use num_bigint::BigUint;
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned};

pub mod array;
pub mod errors;
pub mod reference;

use crate::errors::IndexedMerkleTreeError;

pub const FIELD_SIZE_SUB_ONE: &str =
    "21888242871839275222246405745257275088548364400416034343698204186575808495616";

#[derive(Debug, Default, Clone, Copy)]
pub struct RawIndexedElement<I>
where
    I: Clone + Pod,
{
    pub value: [u8; 32],
    pub next_index: I,
    pub next_value: [u8; 32],
    pub index: I,
}
unsafe impl<I> Pod for RawIndexedElement<I> where I: Pod + Clone {}

#[repr(C)]
pub struct IndexedMerkleTree<'a, H, I, const HEIGHT: usize>
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
        + Pod,
    usize: From<I>,
{
    pub merkle_tree: ConcurrentMerkleTree<'a, H, HEIGHT>,
    pub next_changelog_index: usize,
    pub changelog_size: usize,
    pub changelog: CyclicBoundedVec<'a, RawIndexedElement<I>>,
    _index: PhantomData<I>,
}

pub type IndexedMerkleTree22<'a, H, I> = IndexedMerkleTree<'a, H, I, 22>;
pub type IndexedMerkleTree26<'a, H, I> = IndexedMerkleTree<'a, H, I, 26>;
pub type IndexedMerkleTree32<'a, H, I> = IndexedMerkleTree<'a, H, I, 32>;
pub type IndexedMerkleTree40<'a, H, I> = IndexedMerkleTree<'a, H, I, 40>;

impl<'a, H, I, const HEIGHT: usize> IndexedMerkleTree<'a, H, I, HEIGHT>
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
        + Pod,
    usize: From<I>,
{
    pub fn new(
        height: usize,
        changelog_size: usize,
        roots_size: usize,
        canopy_depth: usize,
        indexed_change_log_size: usize,
    ) -> Result<Self, ConcurrentMerkleTreeError> {
        let merkle_tree = ConcurrentMerkleTree::<H, HEIGHT>::new(
            height,
            changelog_size,
            roots_size,
            canopy_depth,
        )?;
        Ok(Self {
            merkle_tree,
            next_changelog_index: 0,
            changelog_size: indexed_change_log_size,
            changelog: CyclicBoundedVec::with_capacity(indexed_change_log_size),
            _index: PhantomData,
        })
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
        // bytes_indexed_change_log: &'b mut [u8],
        // next_index: usize,
        // cyclic_vec_size: usize,
        // changelog_size: usize,
    ) -> Result<&'b mut Self, ConcurrentMerkleTreeError> {
        let merkle_tree = ConcurrentMerkleTree::<H, HEIGHT>::from_bytes_mut(
            bytes_struct,
            bytes_filled_subtrees,
            bytes_changelog,
            bytes_roots,
            bytes_canopy,
        )?;
        // let changelog = CyclicBoundedVec::from_raw_parts(
        //     bytes_indexed_change_log.as_mut_ptr() as _,
        //     next_index,
        //     cyclic_vec_size,
        //     changelog_size,
        // );

        // Ok(&mut IndexedMerkleTree {
        //     merkle_tree: *(merkle_tree),
        //     next_changelog_index: 0,
        //     changelog_size,
        //     changelog,
        //     _index: PhantomData,
        // })
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
    #[allow(clippy::type_complexity)]
    pub fn patch_low_element(
        &mut self,
        low_element: &IndexedElement<I>,
    ) -> Result<Option<(IndexedElement<I>, [u8; 32])>, IndexedMerkleTreeError> {
        let changelog_element_index = self
            .changelog
            .iter()
            .position(|element| element.index == low_element.index);

        match changelog_element_index {
            Some(changelog_element_index) => {
                let max_usize = usize::MAX;
                // TODO: benchmark whether overwriting or the comparison is more expensive.
                // Removed elements must not be used again.
                if changelog_element_index == max_usize {
                    return Err(IndexedMerkleTreeError::LowElementNotFound);
                }
                let changelog_element = &mut self.changelog[changelog_element_index];
                let patched_element = IndexedElement::<I> {
                    value: BigUint::from_bytes_be(&changelog_element.value),
                    index: changelog_element.index,
                    next_index: changelog_element.next_index,
                };
                // Removing the value:
                // Writing data costs CU thus we just overwrite the index
                // with an impossible value so that it cannot be found.
                changelog_element.index = max_usize
                    .try_into()
                    .map_err(|_| IndexedMerkleTreeError::IntegerOverflow)?;
                // Only use changelog event values, since these originate from an account -> can be trusted
                Ok(Some((patched_element, changelog_element.next_value)))
            }
            None => Ok(None),
        }
    }

    pub fn update(
        &mut self,
        changelog_index: usize,
        new_element: IndexedElement<I>,
        mut low_element: IndexedElement<I>,
        mut low_element_next_value: BigUint,
        low_leaf_proof: &mut BoundedVec<[u8; 32]>,
    ) -> Result<(), IndexedMerkleTreeError> {
        let patched_low_element = self.patch_low_element(&low_element)?;
        // match patched_low_element {
        //     Some((patched_low_element, patched_low_element_next_value)) => {
        //         low_element = patched_low_element;
        //         low_element_next_value = BigUint::from_bytes_be(&patched_low_element_next_value);
        //     }
        //     None => {}
        // }
        if let Some((patched_low_element, patched_low_element_next_value)) = patched_low_element {
            low_element = patched_low_element;
            low_element_next_value = BigUint::from_bytes_be(&patched_low_element_next_value);
        };
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
            if new_element.value >= low_element_next_value {
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
        let old_low_leaf = low_element.hash::<H>(&low_element_next_value)?;
        let new_low_leaf = new_low_element.hash::<H>(&new_element.value)?;

        self.merkle_tree.update(
            changelog_index,
            &old_low_leaf,
            &new_low_leaf,
            low_element.index.into(),
            low_leaf_proof,
            true,
        )?;

        // Append new element.
        let new_leaf = new_element.hash::<H>(&low_element_next_value)?;
        self.merkle_tree.append(&new_leaf)?;

        self.changelog
            .push(RawIndexedElement {
                value: bigint_to_be_bytes_array::<32>(&new_low_element.value).unwrap(),
                next_index: new_low_element.next_index,
                next_value: bigint_to_be_bytes_array::<32>(&new_element.value)?,
                index: new_low_element.index,
            })
            .unwrap();
        self.next_changelog_index = (self.next_changelog_index + 1) % self.changelog_size;
        Ok(())
    }

    /// Initializes the address merkle tree with the given initial value.
    /// The initial value should be a high value since one needs to prove non-inclusion of an address prior to insertion.
    /// Thus, addresses with higher values than the initial value cannot be inserted.
    pub fn initialize_address_merkle_tree(
        address_merkle_tree_inited: &mut IndexedMerkleTree<'a, H, I, HEIGHT>,
        init_value: BigUint,
    ) -> Result<(), IndexedMerkleTreeError> {
        let mut indexed_array = IndexedArray::<H, I, 2>::default();
        let nullifier_bundle = indexed_array.append(&init_value)?;
        let new_low_leaf = nullifier_bundle
            .new_low_element
            .hash::<H>(&nullifier_bundle.new_element.value)?;
        let mut zero_bytes_array = BoundedVec::with_capacity(26);
        for i in 0..address_merkle_tree_inited.merkle_tree.height {
            // : Calling `unwrap()` pushing into this bounded vec cannot panic since the array has enough capacity.
            zero_bytes_array.push(H::zero_bytes()[i]).unwrap();
        }
        address_merkle_tree_inited.merkle_tree.update(
            address_merkle_tree_inited.changelog_index(),
            &H::zero_indexed_leaf(),
            &new_low_leaf,
            0,
            &mut zero_bytes_array,
            true,
        )?;
        let new_leaf = nullifier_bundle
            .new_element
            .hash::<H>(&nullifier_bundle.new_element_next_value)?;
        address_merkle_tree_inited.merkle_tree.append(&new_leaf)?;
        Ok(())
    }
}
