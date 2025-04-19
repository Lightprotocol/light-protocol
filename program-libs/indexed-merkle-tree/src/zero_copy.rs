use std::{
    fmt,
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
};

use light_bounded_vec::{CyclicBoundedVec, CyclicBoundedVecMetadata};
use light_concurrent_merkle_tree::{
    errors::ConcurrentMerkleTreeError,
    offset::zero_copy::{read_array_like_ptr_at, read_ptr_at, write_at},
    zero_copy::{ConcurrentMerkleTreeZeroCopy, ConcurrentMerkleTreeZeroCopyMut},
    ConcurrentMerkleTree,
};
use light_hasher::Hasher;
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned};

use crate::{errors::IndexedMerkleTreeError, IndexedMerkleTree};

#[derive(Debug)]
pub struct IndexedMerkleTreeZeroCopy<'a, H, I, const HEIGHT: usize, const NET_HEIGHT: usize>
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
    pub merkle_tree: mem::ManuallyDrop<IndexedMerkleTree<H, I, HEIGHT, NET_HEIGHT>>,
    // The purpose of this field is ensuring that the wrapper does not outlive
    // the buffer.
    _bytes: &'a [u8],
}

impl<'a, H, I, const HEIGHT: usize, const NET_HEIGHT: usize>
    IndexedMerkleTreeZeroCopy<'a, H, I, HEIGHT, NET_HEIGHT>
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
    /// Returns a zero-copy wrapper of `IndexedMerkleTree` created from the
    /// data in the provided `bytes` buffer.
    pub fn from_bytes_zero_copy(bytes: &'a [u8]) -> Result<Self, IndexedMerkleTreeError> {
        let (merkle_tree, mut offset) =
            ConcurrentMerkleTreeZeroCopy::struct_from_bytes_zero_copy(bytes)?;

        let indexed_changelog_metadata: *mut CyclicBoundedVecMetadata =
            unsafe { read_ptr_at(bytes, &mut offset) };

        let expected_size = IndexedMerkleTree::<H, I, HEIGHT, NET_HEIGHT>::size_in_account(
            merkle_tree.height,
            merkle_tree.changelog.capacity(),
            merkle_tree.roots.capacity(),
            merkle_tree.canopy_depth,
            unsafe { (*indexed_changelog_metadata).capacity() },
        );
        if bytes.len() < expected_size {
            return Err(IndexedMerkleTreeError::ConcurrentMerkleTree(
                ConcurrentMerkleTreeError::BufferSize(expected_size, bytes.len()),
            ));
        }

        let indexed_changelog = unsafe {
            CyclicBoundedVec::from_raw_parts(
                indexed_changelog_metadata,
                read_array_like_ptr_at(
                    bytes,
                    &mut offset,
                    (*indexed_changelog_metadata).capacity(),
                ),
            )
        };

        Ok(Self {
            merkle_tree: mem::ManuallyDrop::new(IndexedMerkleTree {
                merkle_tree,
                indexed_changelog,
                _index: PhantomData,
            }),
            _bytes: bytes,
        })
    }
}

impl<H, I, const HEIGHT: usize, const NET_HEIGHT: usize> Deref
    for IndexedMerkleTreeZeroCopy<'_, H, I, HEIGHT, NET_HEIGHT>
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
    type Target = IndexedMerkleTree<H, I, HEIGHT, NET_HEIGHT>;

    fn deref(&self) -> &Self::Target {
        &self.merkle_tree
    }
}

#[derive(Debug)]
pub struct IndexedMerkleTreeZeroCopyMut<'a, H, I, const HEIGHT: usize, const NET_HEIGHT: usize>(
    IndexedMerkleTreeZeroCopy<'a, H, I, HEIGHT, NET_HEIGHT>,
)
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
    usize: From<I>;

impl<'a, H, I, const HEIGHT: usize, const NET_HEIGHT: usize>
    IndexedMerkleTreeZeroCopyMut<'a, H, I, HEIGHT, NET_HEIGHT>
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
    pub fn from_bytes_zero_copy_mut(bytes: &'a mut [u8]) -> Result<Self, IndexedMerkleTreeError> {
        Ok(Self(IndexedMerkleTreeZeroCopy::from_bytes_zero_copy(
            bytes,
        )?))
    }

    pub fn from_bytes_zero_copy_init(
        bytes: &'a mut [u8],
        height: usize,
        canopy_depth: usize,
        changelog_capacity: usize,
        roots_capacity: usize,
        indexed_changelog_capacity: usize,
    ) -> Result<Self, IndexedMerkleTreeError> {
        let _ = ConcurrentMerkleTreeZeroCopyMut::<H, HEIGHT>::fill_non_dyn_fields_in_buffer(
            bytes,
            height,
            canopy_depth,
            changelog_capacity,
            roots_capacity,
        )?;

        let expected_size = IndexedMerkleTree::<H, I, HEIGHT, NET_HEIGHT>::size_in_account(
            height,
            changelog_capacity,
            roots_capacity,
            canopy_depth,
            indexed_changelog_capacity,
        );
        if bytes.len() < expected_size {
            return Err(IndexedMerkleTreeError::ConcurrentMerkleTree(
                ConcurrentMerkleTreeError::BufferSize(expected_size, bytes.len()),
            ));
        }

        let mut offset = ConcurrentMerkleTree::<H, HEIGHT>::size_in_account(
            height,
            changelog_capacity,
            roots_capacity,
            canopy_depth,
        );

        let indexed_changelog_metadata = CyclicBoundedVecMetadata::new(indexed_changelog_capacity);
        write_at::<CyclicBoundedVecMetadata>(
            bytes,
            &indexed_changelog_metadata.to_le_bytes(),
            &mut offset,
        );

        Self::from_bytes_zero_copy_mut(bytes)
    }
}

impl<H, I, const HEIGHT: usize, const NET_HEIGHT: usize> Deref
    for IndexedMerkleTreeZeroCopyMut<'_, H, I, HEIGHT, NET_HEIGHT>
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
    type Target = IndexedMerkleTree<H, I, HEIGHT, NET_HEIGHT>;

    fn deref(&self) -> &Self::Target {
        &self.0.merkle_tree
    }
}

impl<H, I, const HEIGHT: usize, const NET_HEIGHT: usize> DerefMut
    for IndexedMerkleTreeZeroCopyMut<'_, H, I, HEIGHT, NET_HEIGHT>
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
        &mut self.0.merkle_tree
    }
}

#[cfg(test)]
mod test {
    use light_hasher::{bigint::bigint_to_be_bytes_array, Poseidon};
    use num_bigint::RandBigInt;
    use rand::thread_rng;

    use super::*;

    fn from_bytes_zero_copy<
        const HEIGHT: usize,
        const NET_HEIGHT: usize,
        const CHANGELOG_SIZE: usize,
        const ROOTS: usize,
        const CANOPY_DEPTH: usize,
        const INDEXED_CHANGELOG_SIZE: usize,
        const OPERATIONS: usize,
    >() {
        let mut mt_1 = IndexedMerkleTree::<Poseidon, usize, HEIGHT, NET_HEIGHT>::new(
            HEIGHT,
            CHANGELOG_SIZE,
            ROOTS,
            CANOPY_DEPTH,
            INDEXED_CHANGELOG_SIZE,
        )
        .unwrap();
        mt_1.init().unwrap();

        let mut bytes = vec![
            0u8;
            IndexedMerkleTree::<Poseidon, usize, HEIGHT, NET_HEIGHT>::size_in_account(
                HEIGHT,
                CHANGELOG_SIZE,
                ROOTS,
                CANOPY_DEPTH,
                INDEXED_CHANGELOG_SIZE
            )
        ];

        {
            let mut mt_2 =
                IndexedMerkleTreeZeroCopyMut::<Poseidon, usize, HEIGHT, NET_HEIGHT>::from_bytes_zero_copy_init(
                    &mut bytes,
                    HEIGHT,
                    CANOPY_DEPTH,
                    CHANGELOG_SIZE,
                    ROOTS,
                    INDEXED_CHANGELOG_SIZE,
                )
                .unwrap();
            mt_2.init().unwrap();

            assert_eq!(mt_1, *mt_2);
        }

        let mut rng = thread_rng();

        for _ in 0..OPERATIONS {
            // Reload the tree from bytes on each iteration.
            let mut mt_2 =
                IndexedMerkleTreeZeroCopyMut::<Poseidon, usize, HEIGHT,NET_HEIGHT>::from_bytes_zero_copy_mut(
                    &mut bytes,
                )
                .unwrap();

            let leaf: [u8; 32] = bigint_to_be_bytes_array::<32>(&rng.gen_biguint(248)).unwrap();
            mt_1.append(&leaf).unwrap();
            mt_2.append(&leaf).unwrap();

            assert_eq!(mt_1, *mt_2);
        }
    }

    #[test]
    fn test_from_bytes_zero_copy_26_1400_2400_10_256_1024() {
        const HEIGHT: usize = 26;
        const NET_HEIGHT: usize = 16;
        const CHANGELOG_SIZE: usize = 1400;
        const ROOTS: usize = 2400;
        const CANOPY_DEPTH: usize = 10;
        const INDEXED_CHANGELOG_SIZE: usize = 256;

        const OPERATIONS: usize = 1024;

        from_bytes_zero_copy::<
            HEIGHT,
            NET_HEIGHT,
            CHANGELOG_SIZE,
            ROOTS,
            CANOPY_DEPTH,
            INDEXED_CHANGELOG_SIZE,
            OPERATIONS,
        >()
    }
}
