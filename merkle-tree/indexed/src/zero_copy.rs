use std::mem;

use light_bounded_vec::{BoundedVec, CyclicBoundedVec, Pod};
use light_concurrent_merkle_tree::{
    changelog::ChangelogEntry, errors::ConcurrentMerkleTreeError, ConcurrentMerkleTree,
};
use light_hasher::Hasher;
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned};

use crate::{errors::IndexedMerkleTreeError, IndexedMerkleTree, RawIndexedElement};

#[derive(Debug)]
pub struct IndexedMerkleTreeZeroCopy<'a, H, I, const HEIGHT: usize>
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
    pub merkle_tree: &'a IndexedMerkleTree<'a, H, I, HEIGHT>,
}

pub type IndexedMerkleTreeZeroCopy22<'a, H, I> = IndexedMerkleTreeZeroCopy<'a, H, I, 22>;
pub type IndexedMerkleTreeZeroCopy26<'a, H, I> = IndexedMerkleTreeZeroCopy<'a, H, I, 26>;
pub type IndexedMerkleTreeZeroCopy32<'a, H, I> = IndexedMerkleTreeZeroCopy<'a, H, I, 32>;
pub type IndexedMerkleTreeZeroCopy40<'a, H, I> = IndexedMerkleTreeZeroCopy<'a, H, I, 40>;

impl<'a, H, I, const HEIGHT: usize> IndexedMerkleTreeZeroCopy<'a, H, I, HEIGHT>
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
    pub unsafe fn struct_from_bytes_zero_copy(
        bytes_struct: &'a [u8],
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
        let tree: *const IndexedMerkleTree<'a, H, I, HEIGHT> = bytes_struct.as_ptr() as _;

        Ok(Self {
            merkle_tree: &*tree,
        })
    }

    pub unsafe fn from_bytes_zero_copy(
        bytes_struct: &'a [u8],
        bytes_filled_subtrees: &'a [u8],
        bytes_changelog: &'a [u8],
        bytes_roots: &'a [u8],
        bytes_canopy: &'a [u8],
        bytes_indexed_changelog: &'a [u8],
    ) -> Result<Self, IndexedMerkleTreeError> {
        let tree = IndexedMerkleTreeZeroCopyMut::struct_from_bytes_zero_copy_mut(bytes_struct)?;

        if tree.merkle_tree.merkle_tree.height == 0 {
            return Err(IndexedMerkleTreeError::ConcurrentMerkleTree(
                ConcurrentMerkleTreeError::HeightZero,
            ));
        }
        if tree.merkle_tree.merkle_tree.changelog_capacity == 0 {
            return Err(IndexedMerkleTreeError::ConcurrentMerkleTree(
                ConcurrentMerkleTreeError::ChangelogZero,
            ));
        }

        // Restore the vectors correctly, by pointing them to the appropriate
        // byte slices as underlying data. The most unsafe part of this code.
        // Here be dragons!
        let expected_bytes_filled_subtrees_size =
            mem::size_of::<[u8; 32]>() * tree.merkle_tree.merkle_tree.height;
        if bytes_filled_subtrees.len() != expected_bytes_filled_subtrees_size {
            return Err(IndexedMerkleTreeError::ConcurrentMerkleTree(
                ConcurrentMerkleTreeError::FilledSubtreesBufferSize(
                    expected_bytes_filled_subtrees_size,
                    bytes_filled_subtrees.len(),
                ),
            ));
        }
        tree.merkle_tree.merkle_tree.filled_subtrees = BoundedVec::from_raw_parts(
            bytes_filled_subtrees.as_ptr() as _,
            tree.merkle_tree.merkle_tree.height,
            tree.merkle_tree.merkle_tree.height,
        );

        let expected_bytes_changelog_size = mem::size_of::<ChangelogEntry<HEIGHT>>()
            * tree.merkle_tree.merkle_tree.changelog_capacity;
        if bytes_changelog.len() != expected_bytes_changelog_size {
            return Err(IndexedMerkleTreeError::ConcurrentMerkleTree(
                ConcurrentMerkleTreeError::ChangelogBufferSize(
                    expected_bytes_changelog_size,
                    bytes_changelog.len(),
                ),
            ));
        }
        tree.merkle_tree.merkle_tree.changelog = CyclicBoundedVec::from_raw_parts(
            bytes_changelog.as_ptr() as _,
            tree.merkle_tree.merkle_tree.current_changelog_index + 1,
            tree.merkle_tree.merkle_tree.changelog_length,
            tree.merkle_tree.merkle_tree.changelog_capacity,
        );

        let expected_bytes_roots_size =
            mem::size_of::<[u8; 32]>() * tree.merkle_tree.merkle_tree.roots_capacity;
        if bytes_roots.len() != expected_bytes_roots_size {
            return Err(IndexedMerkleTreeError::ConcurrentMerkleTree(
                ConcurrentMerkleTreeError::RootBufferSize(
                    expected_bytes_roots_size,
                    bytes_roots.len(),
                ),
            ));
        }
        tree.merkle_tree.merkle_tree.roots =
            ConcurrentMerkleTree::<'a, H, HEIGHT>::roots_from_bytes(
                bytes_roots,
                tree.merkle_tree.merkle_tree.current_root_index + 1,
                tree.merkle_tree.merkle_tree.roots_length,
                tree.merkle_tree.merkle_tree.roots_capacity,
            )?;

        let canopy_size = ConcurrentMerkleTree::<'a, H, HEIGHT>::canopy_size(
            tree.merkle_tree.merkle_tree.canopy_depth,
        );
        let expected_canopy_size = mem::size_of::<[u8; 32]>() * canopy_size;
        if bytes_canopy.len() != expected_canopy_size {
            return Err(IndexedMerkleTreeError::ConcurrentMerkleTree(
                ConcurrentMerkleTreeError::CanopyBufferSize(
                    expected_canopy_size,
                    bytes_canopy.len(),
                ),
            ));
        }
        tree.merkle_tree.merkle_tree.canopy =
            BoundedVec::from_raw_parts(bytes_canopy.as_ptr() as _, canopy_size, canopy_size);

        let expected_bytes_indexed_changelog_size =
            mem::size_of::<RawIndexedElement<I>>() * tree.merkle_tree.changelog.capacity();
        if bytes_indexed_changelog.len() != expected_bytes_indexed_changelog_size {
            return Err(IndexedMerkleTreeError::ChangelogBufferSize(
                expected_bytes_indexed_changelog_size,
                bytes_indexed_changelog.len(),
            ));
        }
        tree.merkle_tree.changelog = CyclicBoundedVec::from_raw_parts(
            bytes_indexed_changelog.as_ptr() as _,
            tree.merkle_tree.changelog.next_index(),
            tree.merkle_tree.changelog.len(),
            tree.merkle_tree.changelog.capacity(),
        );

        Ok(IndexedMerkleTreeZeroCopy {
            merkle_tree: tree.merkle_tree,
        })
    }
}

#[derive(Debug)]
pub struct IndexedMerkleTreeZeroCopyMut<'a, H, I, const HEIGHT: usize>
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
    pub merkle_tree: &'a mut IndexedMerkleTree<'a, H, I, HEIGHT>,
}

pub type IndexedMerkleTreeZeroCopyMut22<'a, H, I> = IndexedMerkleTreeZeroCopyMut<'a, H, I, 22>;
pub type IndexedMerkleTreeZeroCopyMut26<'a, H, I> = IndexedMerkleTreeZeroCopyMut<'a, H, I, 26>;
pub type IndexedMerkleTreeZeroCopyMut32<'a, H, I> = IndexedMerkleTreeZeroCopyMut<'a, H, I, 32>;
pub type IndexedMerkleTreeZeroCopyMut40<'a, H, I> = IndexedMerkleTreeZeroCopyMut<'a, H, I, 40>;

impl<'a, H, I, const HEIGHT: usize> IndexedMerkleTreeZeroCopyMut<'a, H, I, HEIGHT>
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
    // TODO(vadorovsky): Add a non-mut method: `from_bytes_zero_copy`.

    pub unsafe fn struct_from_bytes_zero_copy_mut(
        bytes_struct: &'a [u8],
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
        let tree: *mut IndexedMerkleTree<'a, H, I, HEIGHT> = bytes_struct.as_ptr() as _;

        Ok(Self {
            merkle_tree: &mut *tree,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub unsafe fn fill_vectors_mut(
        &mut self,
        bytes_filled_subtrees: &'a mut [u8],
        bytes_changelog: &'a mut [u8],
        bytes_roots: &'a mut [u8],
        bytes_canopy: &'a mut [u8],
        subtrees_length: usize,
        changelog_next_index: usize,
        changelog_length: usize,
        roots_next_index: usize,
        roots_length: usize,
        canopy_length: usize,
        bytes_indexed_changelog: &'a mut [u8],
        indexed_changelog_next_index: usize,
        indexed_changelog_length: usize,
        indexed_changelog_capacity: usize,
    ) -> Result<(), IndexedMerkleTreeError> {
        self.merkle_tree.merkle_tree.fill_vectors_mut(
            bytes_filled_subtrees,
            bytes_changelog,
            bytes_roots,
            bytes_canopy,
            subtrees_length,
            changelog_next_index,
            changelog_length,
            roots_next_index,
            roots_length,
            canopy_length,
        )?;

        #[cfg(feture = "solana")]
        solana_program::msg!(
            "changelog capacity: {}",
            self.merkle_tree.changelog_capacity
        );
        let expected_bytes_indexed_changelog_size =
            mem::size_of::<RawIndexedElement<I>>() * self.merkle_tree.changelog.capacity();
        #[cfg(feture = "solana")]
        solana_program::msg!(
            "expected_bytes_indexed_changelog_size: {}",
            expected_bytes_indexed_changelog_size
        );
        if bytes_indexed_changelog.len() != expected_bytes_indexed_changelog_size {
            return Err(IndexedMerkleTreeError::ChangelogBufferSize(
                expected_bytes_indexed_changelog_size,
                bytes_indexed_changelog.len(),
            ));
        }
        self.merkle_tree.changelog = CyclicBoundedVec::from_raw_parts(
            bytes_indexed_changelog.as_mut_ptr() as _,
            indexed_changelog_next_index,
            indexed_changelog_length,
            indexed_changelog_capacity,
        );

        Ok(())
    }

    pub unsafe fn from_bytes_zero_copy_init(
        bytes_struct: &'a mut [u8],
        bytes_filled_subtrees: &'a mut [u8],
        bytes_changelog: &'a mut [u8],
        bytes_roots: &'a mut [u8],
        bytes_canopy: &'a mut [u8],
        height: usize,
        changelog_size: usize,
        roots_size: usize,
        canopy_depth: usize,
        bytes_indexed_changelog: &'a mut [u8],
        indexed_changelog_size: usize,
    ) -> Result<Self, IndexedMerkleTreeError> {
        #[cfg(feture = "solana")]
        solana_program::msg!("compression!");
        let mut tree = Self::struct_from_bytes_zero_copy_mut(bytes_struct)?;

        tree.merkle_tree.merkle_tree.height = height;

        tree.merkle_tree.merkle_tree.changelog_capacity = changelog_size;
        tree.merkle_tree.merkle_tree.changelog_length = 0;
        tree.merkle_tree.merkle_tree.current_changelog_index = 0;

        tree.merkle_tree.merkle_tree.roots_capacity = roots_size;
        tree.merkle_tree.merkle_tree.roots_length = 0;
        tree.merkle_tree.merkle_tree.current_root_index = 0;

        tree.merkle_tree.merkle_tree.canopy_depth = canopy_depth;

        tree.merkle_tree.changelog = CyclicBoundedVec::with_capacity(indexed_changelog_size);

        tree.fill_vectors_mut(
            bytes_filled_subtrees,
            bytes_changelog,
            bytes_roots,
            bytes_canopy,
            0,
            0,
            0,
            0,
            0,
            0,
            bytes_indexed_changelog,
            0,
            0,
            indexed_changelog_size,
        )?;

        Ok(tree)
    }

    /// Casts byte slices into `IndexedMerkleTreeZeroCopy`.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. Ensuring the size and alignment of the byte
    /// slices is the caller's responsibility.
    pub unsafe fn from_bytes_zero_copy_mut(
        bytes_struct: &'a mut [u8],
        bytes_filled_subtrees: &'a mut [u8],
        bytes_changelog: &'a mut [u8],
        bytes_roots: &'a mut [u8],
        bytes_canopy: &'a mut [u8],
        bytes_indexed_changelog: &'a mut [u8],
    ) -> Result<Self, IndexedMerkleTreeError> {
        let mut tree = Self::struct_from_bytes_zero_copy_mut(bytes_struct)?;

        tree.fill_vectors_mut(
            bytes_filled_subtrees,
            bytes_changelog,
            bytes_roots,
            bytes_canopy,
            tree.merkle_tree.merkle_tree.height,
            tree.merkle_tree.merkle_tree.current_changelog_index + 1,
            tree.merkle_tree.merkle_tree.changelog_length,
            tree.merkle_tree.merkle_tree.current_root_index + 1,
            tree.merkle_tree.merkle_tree.roots_length,
            ConcurrentMerkleTree::<'a, H, HEIGHT>::canopy_size(
                tree.merkle_tree.merkle_tree.canopy_depth,
            ),
            bytes_indexed_changelog,
            tree.merkle_tree.changelog.next_index(),
            tree.merkle_tree.changelog.len(),
            tree.merkle_tree.changelog.capacity(),
        )?;

        Ok(tree)
    }
}

#[cfg(test)]
mod test {
    use light_hasher::Poseidon;

    use super::*;

    #[test]
    fn test_from_bytes_zero_copy_init() {
        let mut bytes_struct = [0u8; 296];
        let mut bytes_filled_subtrees = [0u8; 832];
        let mut bytes_changelog = [0u8; 1220800];
        let mut bytes_roots = [0u8; 76800];
        let mut bytes_canopy = [0u8; 65472];

        const HEIGHT: usize = 26;
        const CHANGELOG_SIZE: usize = 1400;
        const ROOTS: usize = 2400;
        const CANOPY_DEPTH: usize = 10;

        let mut bytes_indexed_changelog = [0u8; 20480];

        const INDEXED_CHANGELOG_SIZE: usize = 256;

        let mt = unsafe {
            IndexedMerkleTreeZeroCopyMut::<Poseidon, u16, HEIGHT>::from_bytes_zero_copy_init(
                &mut bytes_struct,
                &mut bytes_filled_subtrees,
                &mut bytes_changelog,
                &mut bytes_roots,
                &mut bytes_canopy,
                HEIGHT,
                CHANGELOG_SIZE,
                ROOTS,
                CANOPY_DEPTH,
                &mut bytes_indexed_changelog,
                INDEXED_CHANGELOG_SIZE,
            )
            .unwrap()
        };
        mt.merkle_tree.init().unwrap();

        assert_eq!(
            mt.merkle_tree.merkle_tree.root().unwrap(),
            [
                31, 216, 114, 222, 104, 109, 25, 228, 3, 94, 104, 27, 124, 142, 79, 197, 7, 102,
                233, 55, 135, 141, 70, 48, 130, 255, 202, 209, 122, 217, 210, 162
            ]
        );
    }
}
