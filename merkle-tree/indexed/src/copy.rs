use std::{marker::PhantomData, mem, slice};

use light_bounded_vec::{BoundedVec, CyclicBoundedVec, Pod};
use light_concurrent_merkle_tree::{
    changelog::ChangelogEntry, errors::ConcurrentMerkleTreeError, ConcurrentMerkleTree,
};
use light_hasher::Hasher;
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned};

use crate::{errors::IndexedMerkleTreeError, IndexedMerkleTree, RawIndexedElement};

#[derive(Debug)]
pub struct IndexedMerkleTreeCopy<'a, H, I, const HEIGHT: usize>(
    pub IndexedMerkleTree<'a, H, I, HEIGHT>,
)
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
    usize: From<I>;

pub type IndexedMerkleTreeCopy22<'a, H, I> = IndexedMerkleTreeCopy<'a, H, I, 22>;
pub type IndexedMerkleTreeCopy26<'a, H, I> = IndexedMerkleTreeCopy<'a, H, I, 26>;
pub type IndexedMerkleTreeCopy32<'a, H, I> = IndexedMerkleTreeCopy<'a, H, I, 32>;
pub type IndexedMerkleTreeCopy40<'a, H, I> = IndexedMerkleTreeCopy<'a, H, I, 40>;

impl<'a, H, I, const HEIGHT: usize> IndexedMerkleTreeCopy<'a, H, I, HEIGHT>
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
    pub unsafe fn copy_from_bytes(
        bytes_struct: &[u8],
        bytes_filled_subtrees: &[u8],
        bytes_changelog: &[u8],
        bytes_roots: &[u8],
        bytes_canopy: &[u8],
        bytes_indexed_changelog: &'a [u8],
    ) -> Result<Self, IndexedMerkleTreeError> {
        let expected_bytes_struct_size = mem::size_of::<IndexedMerkleTree<'a, H, I, HEIGHT>>();
        if bytes_struct.len() != expected_bytes_struct_size {
            return Err(IndexedMerkleTreeError::ConcurrentMerkleTree(
                ConcurrentMerkleTreeError::StructBufferSize(
                    expected_bytes_struct_size,
                    bytes_struct.len(),
                ),
            ));
        }
        let struct_ref: *mut IndexedMerkleTree<'a, H, I, HEIGHT> = bytes_struct.as_ptr() as _;

        let mut merkle_tree = unsafe {
            ConcurrentMerkleTree {
                height: (*struct_ref).merkle_tree.height,

                changelog_capacity: (*struct_ref).merkle_tree.changelog_capacity,
                changelog_length: (*struct_ref).merkle_tree.changelog_length,
                current_changelog_index: (*struct_ref).merkle_tree.current_changelog_index,

                roots_capacity: (*struct_ref).merkle_tree.roots_capacity,
                roots_length: (*struct_ref).merkle_tree.roots_length,
                current_root_index: (*struct_ref).merkle_tree.current_root_index,

                canopy_depth: (*struct_ref).merkle_tree.canopy_depth,

                next_index: (*struct_ref).merkle_tree.next_index,
                sequence_number: (*struct_ref).merkle_tree.sequence_number,
                rightmost_leaf: (*struct_ref).merkle_tree.rightmost_leaf,

                filled_subtrees: BoundedVec::with_capacity((*struct_ref).merkle_tree.height),
                changelog: CyclicBoundedVec::with_capacity(
                    (*struct_ref).merkle_tree.changelog_capacity,
                ),
                roots: CyclicBoundedVec::with_capacity((*struct_ref).merkle_tree.roots_capacity),
                canopy: BoundedVec::with_capacity(ConcurrentMerkleTree::<H, HEIGHT>::canopy_size(
                    (*struct_ref).merkle_tree.canopy_depth,
                )),

                _hasher: PhantomData,
            }
        };

        let expected_bytes_filled_subtrees_size =
            mem::size_of::<[u8; 32]>() * (*struct_ref).merkle_tree.height;
        if bytes_filled_subtrees.len() != expected_bytes_filled_subtrees_size {
            return Err(ConcurrentMerkleTreeError::FilledSubtreesBufferSize(
                expected_bytes_filled_subtrees_size,
                bytes_filled_subtrees.len(),
            )
            .into());
        }
        let filled_subtrees: &[[u8; 32]] = slice::from_raw_parts(
            bytes_filled_subtrees.as_ptr() as *const _,
            (*struct_ref).merkle_tree.height,
        );
        for subtree in filled_subtrees.iter() {
            merkle_tree.filled_subtrees.push(*subtree)?;
        }

        let expected_bytes_changelog_size =
            mem::size_of::<ChangelogEntry<HEIGHT>>() * (*struct_ref).merkle_tree.changelog_capacity;
        if bytes_changelog.len() != expected_bytes_changelog_size {
            return Err(ConcurrentMerkleTreeError::ChangelogBufferSize(
                expected_bytes_changelog_size,
                bytes_changelog.len(),
            )
            .into());
        }
        let changelog: &[ChangelogEntry<HEIGHT>] = slice::from_raw_parts(
            bytes_changelog.as_ptr() as *const _,
            (*struct_ref).merkle_tree.changelog_length,
        );
        for changelog_entry in changelog.iter() {
            merkle_tree.changelog.push(changelog_entry.clone())?;
        }

        let expected_bytes_roots_size =
            mem::size_of::<[u8; 32]>() * (*struct_ref).merkle_tree.roots_capacity;
        if bytes_roots.len() != expected_bytes_roots_size {
            return Err(ConcurrentMerkleTreeError::RootBufferSize(
                expected_bytes_roots_size,
                bytes_roots.len(),
            )
            .into());
        }
        let roots: &[[u8; 32]] = slice::from_raw_parts(
            bytes_roots.as_ptr() as *const _,
            (*struct_ref).merkle_tree.roots_length,
        );
        for root in roots.iter() {
            merkle_tree.roots.push(*root)?;
        }

        let canopy_size =
            ConcurrentMerkleTree::<H, HEIGHT>::canopy_size((*struct_ref).merkle_tree.canopy_depth);
        let expected_canopy_size = mem::size_of::<[u8; 32]>() * canopy_size;
        if bytes_canopy.len() != expected_canopy_size {
            return Err(ConcurrentMerkleTreeError::CanopyBufferSize(
                expected_canopy_size,
                bytes_canopy.len(),
            )
            .into());
        }
        let canopy: &[[u8; 32]] =
            slice::from_raw_parts(bytes_canopy.as_ptr() as *const _, canopy_size);
        for node in canopy.iter() {
            merkle_tree.canopy.push(*node)?;
        }

        let mut indexed_merkle_tree = unsafe {
            IndexedMerkleTree {
                merkle_tree,
                changelog: CyclicBoundedVec::with_capacity((*struct_ref).changelog.capacity()),
                _index: PhantomData,
            }
        };

        let expected_bytes_indexed_changelog_size =
            mem::size_of::<RawIndexedElement<I>>() * (*struct_ref).changelog.capacity();
        if bytes_indexed_changelog.len() != expected_bytes_indexed_changelog_size {
            return Err(IndexedMerkleTreeError::ChangelogBufferSize(
                expected_bytes_indexed_changelog_size,
                bytes_indexed_changelog.len(),
            ));
        }
        let indexed_changelog: &[RawIndexedElement<I>] = slice::from_raw_parts(
            bytes_indexed_changelog.as_ptr() as *const _,
            (*struct_ref).changelog.len(),
        );
        for changelog_entry in indexed_changelog.iter() {
            indexed_merkle_tree.changelog.push(*changelog_entry)?;
        }

        Ok(IndexedMerkleTreeCopy(indexed_merkle_tree))
    }
}
