use std::{
    fmt,
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
};

use array::{IndexedArray, IndexedElement};
use changelog::IndexedChangelogEntry;
use light_bounded_vec::{BoundedVec, CyclicBoundedVec, CyclicBoundedVecMetadata};
use light_concurrent_merkle_tree::{
    errors::ConcurrentMerkleTreeError,
    event::{IndexedMerkleTreeUpdate, RawIndexedElement},
    light_hasher::Hasher,
    ConcurrentMerkleTree,
};
use light_hasher::bigint::bigint_to_be_bytes_array;
use num_bigint::BigUint;
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned};

pub mod array;
pub mod changelog;
pub mod copy;
pub mod errors;
pub mod reference;
pub mod zero_copy;

use crate::errors::IndexedMerkleTreeError;

pub const HIGHEST_ADDRESS_PLUS_ONE: &str =
    "452312848583266388373324160190187140051835877600158453279131187530910662655";

#[derive(Debug)]
#[repr(C)]
pub struct IndexedMerkleTree<H, I, const HEIGHT: usize, const NET_HEIGHT: usize>
where
    H: Hasher,
    I: CheckedAdd
        + CheckedSub
        + Copy
        + Clone
        + fmt::Debug
        + PartialOrd
        + ToBytes
        + TryFrom<usize>
        + Unsigned,
    usize: From<I>,
{
    pub merkle_tree: ConcurrentMerkleTree<H, HEIGHT>,
    pub indexed_changelog: CyclicBoundedVec<IndexedChangelogEntry<I, NET_HEIGHT>>,

    _index: PhantomData<I>,
}

pub type IndexedMerkleTree26<H, I> = IndexedMerkleTree<H, I, 26, 16>;

impl<H, I, const HEIGHT: usize, const NET_HEIGHT: usize> IndexedMerkleTree<H, I, HEIGHT, NET_HEIGHT>
where
    H: Hasher,
    I: CheckedAdd
        + CheckedSub
        + Copy
        + Clone
        + fmt::Debug
        + PartialOrd
        + ToBytes
        + TryFrom<usize>
        + Unsigned,
    usize: From<I>,
{
    /// Size of the struct **without** dynamically sized fields (`BoundedVec`,
    /// `CyclicBoundedVec`).
    pub fn non_dyn_fields_size() -> usize {
        ConcurrentMerkleTree::<H, HEIGHT>::non_dyn_fields_size()
            // indexed_changelog (metadata)
            + mem::size_of::<CyclicBoundedVecMetadata>()
    }

    // TODO(vadorovsky): Make a macro for that.
    pub fn size_in_account(
        height: usize,
        changelog_size: usize,
        roots_size: usize,
        canopy_depth: usize,
        indexed_changelog_size: usize,
    ) -> usize {
        ConcurrentMerkleTree::<H, HEIGHT>::size_in_account(
            height,
            changelog_size,
            roots_size,
            canopy_depth,
        )
        // indexed_changelog (metadata)
        + mem::size_of::<CyclicBoundedVecMetadata>()
        // indexed_changelog
        + mem::size_of::<IndexedChangelogEntry<I, NET_HEIGHT>>() * indexed_changelog_size
    }

    pub fn new(
        height: usize,
        changelog_size: usize,
        roots_size: usize,
        canopy_depth: usize,
        indexed_changelog_size: usize,
    ) -> Result<Self, ConcurrentMerkleTreeError> {
        let merkle_tree = ConcurrentMerkleTree::<H, HEIGHT>::new(
            height,
            changelog_size,
            roots_size,
            canopy_depth,
        )?;
        Ok(Self {
            merkle_tree,
            indexed_changelog: CyclicBoundedVec::with_capacity(indexed_changelog_size),
            _index: PhantomData,
        })
    }

    pub fn init(&mut self) -> Result<(), IndexedMerkleTreeError> {
        self.merkle_tree.init()?;

        // Append the first low leaf, which has value 0 and does not point
        // to any other leaf yet.
        // This low leaf is going to be updated during the first `update`
        // operation.
        self.merkle_tree.append(&H::zero_indexed_leaf())?;

        // Emit first changelog entries.
        let element = RawIndexedElement {
            value: [0_u8; 32],
            next_index: I::zero(),
            next_value: [0_u8; 32],
            index: I::zero(),
        };
        let changelog_entry = IndexedChangelogEntry {
            element,
            proof: H::zero_bytes()[..NET_HEIGHT].try_into().unwrap(),
            changelog_index: 0,
        };
        self.indexed_changelog.push(changelog_entry.clone());
        self.indexed_changelog.push(changelog_entry);

        Ok(())
    }

    /// Add the hightest element with a maximum value allowed by the prime
    /// field.
    ///
    /// Initializing an indexed Merkle tree not only with the lowest element
    /// (mandatory for the IMT algorithm to work), but also the highest element,
    /// makes non-inclusion proofs easier - there is no special case needed for
    /// the first insertion.
    ///
    /// However, it comes with a tradeoff - the space available in the tree
    /// becomes lower by 1.
    pub fn add_highest_element(&mut self) -> Result<(), IndexedMerkleTreeError> {
        let mut indexed_array = IndexedArray::<H, I>::default();
        let element_bundle = indexed_array.init()?;
        let new_low_leaf = element_bundle
            .new_low_element
            .hash::<H>(&element_bundle.new_element.value)?;

        let mut proof = BoundedVec::with_capacity(self.merkle_tree.height);
        for i in 0..self.merkle_tree.height - self.merkle_tree.canopy_depth {
            // PANICS: Calling `unwrap()` pushing into this bounded vec
            // cannot panic since it has enough capacity.
            proof.push(H::zero_bytes()[i]).unwrap();
        }

        let (changelog_index, _) = self.merkle_tree.update(
            self.changelog_index(),
            &H::zero_indexed_leaf(),
            &new_low_leaf,
            0,
            &mut proof,
        )?;

        // Emit changelog for low element.
        let low_element = RawIndexedElement {
            value: bigint_to_be_bytes_array::<32>(&element_bundle.new_low_element.value)?,
            next_index: element_bundle.new_low_element.next_index,
            next_value: bigint_to_be_bytes_array::<32>(&element_bundle.new_element.value)?,
            index: element_bundle.new_low_element.index,
        };

        let low_element_changelog_entry = IndexedChangelogEntry {
            element: low_element,
            proof: H::zero_bytes()[..NET_HEIGHT].try_into().unwrap(),
            changelog_index,
        };
        self.indexed_changelog.push(low_element_changelog_entry);

        let new_leaf = element_bundle
            .new_element
            .hash::<H>(&element_bundle.new_element_next_value)?;
        let mut proof = BoundedVec::with_capacity(self.height);
        let (changelog_index, _) = self.merkle_tree.append_with_proof(&new_leaf, &mut proof)?;

        // Emit changelog for new element.
        let new_element = RawIndexedElement {
            value: bigint_to_be_bytes_array::<32>(&element_bundle.new_element.value)?,
            next_index: element_bundle.new_element.next_index,
            next_value: [0_u8; 32],
            index: element_bundle.new_element.index,
        };
        let new_element_changelog_entry = IndexedChangelogEntry {
            element: new_element,
            proof: proof.as_slice()[..NET_HEIGHT].try_into().unwrap(),
            changelog_index,
        };

        self.indexed_changelog.push(new_element_changelog_entry);

        Ok(())
    }

    pub fn indexed_changelog_index(&self) -> usize {
        self.indexed_changelog.last_index()
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

    /// Iterates over indexed changelog and every time an entry corresponding
    /// to the provided `low_element` is found, it patches:
    ///
    /// * Changelog index - indexed changelog entries contain corresponding
    ///   changelog indices.
    /// * New element - changes might impact the `next_index` field, which in
    ///   such case is updated.
    /// * Low element - it might completely change if a change introduced an
    ///   element in our range.
    /// * Merkle proof.
    #[allow(clippy::type_complexity)]
    pub fn patch_elements_and_proof(
        &mut self,
        indexed_changelog_index: usize,
        changelog_index: &mut usize,
        new_element: &mut IndexedElement<I>,
        low_element: &mut IndexedElement<I>,
        low_element_next_value: &mut BigUint,
        low_leaf_proof: &mut BoundedVec<[u8; 32]>,
    ) -> Result<(), IndexedMerkleTreeError> {
        let next_indexed_changelog_indices: Vec<usize> = self
            .indexed_changelog
            .iter_from(indexed_changelog_index)?
            .skip(1)
            .enumerate()
            .filter_map(|(index, changelog_entry)| {
                if changelog_entry.element.index == low_element.index {
                    Some((indexed_changelog_index + 1 + index) % self.indexed_changelog.len())
                } else {
                    None
                }
            })
            .collect();

        let mut new_low_element = None;

        for next_indexed_changelog_index in next_indexed_changelog_indices {
            let changelog_entry = &mut self.indexed_changelog[next_indexed_changelog_index];

            let next_element_value = BigUint::from_bytes_be(&changelog_entry.element.next_value);
            if next_element_value < new_element.value {
                // If the next element is lower than the current element, it means
                // that it should become the low element.
                //
                // Save it and break the loop.
                new_low_element = Some((
                    (next_indexed_changelog_index + 1) % self.indexed_changelog.len(),
                    next_element_value,
                ));
                break;
            }

            // Patch the changelog index.
            *changelog_index = changelog_entry.changelog_index;

            // Patch the `next_index` of `new_element`.
            new_element.next_index = changelog_entry.element.next_index;

            // Patch the element.
            low_element.update_from_raw_element(&changelog_entry.element);
            // Patch the next value.
            *low_element_next_value = BigUint::from_bytes_be(&changelog_entry.element.next_value);
            // Patch the proof.
            for i in 0..low_leaf_proof.len() {
                low_leaf_proof[i] = changelog_entry.proof[i];
            }
        }

        // If we found a new low element.
        if let Some((new_low_element_changelog_index, new_low_element)) = new_low_element {
            let new_low_element_changelog_entry =
                &self.indexed_changelog[new_low_element_changelog_index];
            *changelog_index = new_low_element_changelog_entry.changelog_index;
            *low_element = IndexedElement {
                index: new_low_element_changelog_entry.element.index,
                value: new_low_element.clone(),
                next_index: new_low_element_changelog_entry.element.next_index,
            };

            for i in 0..low_leaf_proof.len() {
                low_leaf_proof[i] = new_low_element_changelog_entry.proof[i];
            }
            new_element.next_index = low_element.next_index;

            // Start the patching process from scratch for the new low element.
            return self.patch_elements_and_proof(
                new_low_element_changelog_index,
                changelog_index,
                new_element,
                low_element,
                low_element_next_value,
                low_leaf_proof,
            );
        }

        Ok(())
    }

    pub fn update(
        &mut self,
        mut changelog_index: usize,
        indexed_changelog_index: usize,
        new_element_value: BigUint,
        mut low_element: IndexedElement<I>,
        mut low_element_next_value: BigUint,
        low_leaf_proof: &mut BoundedVec<[u8; 32]>,
    ) -> Result<IndexedMerkleTreeUpdate<I>, IndexedMerkleTreeError> {
        let mut new_element = IndexedElement {
            index: I::try_from(self.merkle_tree.next_index())
                .map_err(|_| IndexedMerkleTreeError::IntegerOverflow)?,
            value: new_element_value,
            next_index: low_element.next_index,
        };
        println!("low_element: {:?}", low_element);

        self.patch_elements_and_proof(
            indexed_changelog_index,
            &mut changelog_index,
            &mut new_element,
            &mut low_element,
            &mut low_element_next_value,
            low_leaf_proof,
        )?;
        println!("patched low_element: {:?}", low_element);
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
        let new_low_element = IndexedElement::<I> {
            index: low_element.index,
            value: low_element.value.clone(),
            next_index: new_element.index,
        };
        // Update low element. If the `old_low_element` does not belong to the
        // tree, validating the proof is going to fail.
        let old_low_leaf = low_element.hash::<H>(&low_element_next_value)?;

        let new_low_leaf = new_low_element.hash::<H>(&new_element.value)?;

        let (new_changelog_index, _) = self.merkle_tree.update(
            changelog_index,
            &old_low_leaf,
            &new_low_leaf,
            low_element.index.into(),
            low_leaf_proof,
        )?;

        // Emit changelog entry for low element.
        let new_low_element = RawIndexedElement {
            value: bigint_to_be_bytes_array::<32>(&new_low_element.value).unwrap(),
            next_index: new_low_element.next_index,
            next_value: bigint_to_be_bytes_array::<32>(&new_element.value)?,
            index: new_low_element.index,
        };
        let low_element_changelog_entry = IndexedChangelogEntry {
            element: new_low_element,
            proof: low_leaf_proof.as_slice()[..NET_HEIGHT].try_into().unwrap(),
            changelog_index: new_changelog_index,
        };

        self.indexed_changelog.push(low_element_changelog_entry);

        // New element is always the newest one in the tree. Since we
        // support concurrent updates, the index provided by the caller
        // might be outdated. Let's just use the latest index indicated
        // by the tree.
        new_element.index =
            I::try_from(self.next_index()).map_err(|_| IndexedMerkleTreeError::IntegerOverflow)?;

        // Append new element.
        let mut proof = BoundedVec::with_capacity(self.height);
        let new_leaf = new_element.hash::<H>(&low_element_next_value)?;
        let (new_changelog_index, _) = self.merkle_tree.append_with_proof(&new_leaf, &mut proof)?;

        // Prepare raw new element to save in changelog.
        let raw_new_element = RawIndexedElement {
            value: bigint_to_be_bytes_array::<32>(&new_element.value).unwrap(),
            next_index: new_element.next_index,
            next_value: bigint_to_be_bytes_array::<32>(&low_element_next_value)?,
            index: new_element.index,
        };

        // Emit changelog entry for new element.
        let new_element_changelog_entry = IndexedChangelogEntry {
            element: raw_new_element,
            proof: proof.as_slice()[..NET_HEIGHT].try_into().unwrap(),
            changelog_index: new_changelog_index,
        };
        self.indexed_changelog.push(new_element_changelog_entry);

        let output = IndexedMerkleTreeUpdate {
            new_low_element,
            new_low_element_hash: new_low_leaf,
            new_high_element: raw_new_element,
            new_high_element_hash: new_leaf,
        };

        Ok(output)
    }
}

impl<H, I, const HEIGHT: usize, const NET_HEIGHT: usize> Deref
    for IndexedMerkleTree<H, I, HEIGHT, NET_HEIGHT>
where
    H: Hasher,
    I: CheckedAdd
        + CheckedSub
        + Copy
        + Clone
        + fmt::Debug
        + PartialOrd
        + ToBytes
        + TryFrom<usize>
        + Unsigned,
    usize: From<I>,
{
    type Target = ConcurrentMerkleTree<H, HEIGHT>;

    fn deref(&self) -> &Self::Target {
        &self.merkle_tree
    }
}

impl<H, I, const HEIGHT: usize, const NET_HEIGHT: usize> DerefMut
    for IndexedMerkleTree<H, I, HEIGHT, NET_HEIGHT>
where
    H: Hasher,
    I: CheckedAdd
        + CheckedSub
        + Copy
        + Clone
        + fmt::Debug
        + PartialOrd
        + ToBytes
        + TryFrom<usize>
        + Unsigned,
    usize: From<I>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.merkle_tree
    }
}

impl<H, I, const HEIGHT: usize, const NET_HEIGHT: usize> PartialEq
    for IndexedMerkleTree<H, I, HEIGHT, NET_HEIGHT>
where
    H: Hasher,
    I: CheckedAdd
        + CheckedSub
        + Copy
        + Clone
        + fmt::Debug
        + PartialOrd
        + ToBytes
        + TryFrom<usize>
        + Unsigned,
    usize: From<I>,
{
    fn eq(&self, other: &Self) -> bool {
        self.merkle_tree.eq(&other.merkle_tree)
            && self
                .indexed_changelog
                .capacity()
                .eq(&other.indexed_changelog.capacity())
            && self
                .indexed_changelog
                .len()
                .eq(&other.indexed_changelog.len())
            && self
                .indexed_changelog
                .first_index()
                .eq(&other.indexed_changelog.first_index())
            && self
                .indexed_changelog
                .last_index()
                .eq(&other.indexed_changelog.last_index())
            && self.indexed_changelog.eq(&other.indexed_changelog)
    }
}
