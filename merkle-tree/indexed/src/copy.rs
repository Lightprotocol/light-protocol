use std::{marker::PhantomData, ops::Deref};

use light_bounded_vec::CyclicBoundedVecMetadata;
use light_concurrent_merkle_tree::{
    copy::ConcurrentMerkleTreeCopy, errors::ConcurrentMerkleTreeError,
};
use light_hasher::Hasher;
use light_utils::offset::{read_cyclic_bounded_vec_at, read_value_at};
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned};

use crate::{errors::IndexedMerkleTreeError, IndexedMerkleTree};

#[derive(Debug)]
pub struct IndexedMerkleTreeCopy<H, I, const HEIGHT: usize>(IndexedMerkleTree<H, I, HEIGHT>)
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>;

impl<H, I, const HEIGHT: usize> IndexedMerkleTreeCopy<H, I, HEIGHT>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>,
{
    /// Casts a byte slice into wrapped `IndexedMerkleTree` structure reference,
    /// including dynamic fields.
    ///
    /// # Purpose
    ///
    /// This method is meant to be used mostly in Solana programs, where memory
    /// constraints are tight and we want to make sure no data is copied.
    pub fn from_bytes_copy(bytes: &[u8]) -> Result<Self, IndexedMerkleTreeError> {
        let (merkle_tree, mut offset) =
            ConcurrentMerkleTreeCopy::<H, HEIGHT>::struct_from_bytes_copy(bytes)?;

        let indexed_changelog_metadata: CyclicBoundedVecMetadata =
            unsafe { read_value_at(bytes, &mut offset) };

        let expected_size = IndexedMerkleTree::<H, I, HEIGHT>::size_in_account(
            merkle_tree.height,
            merkle_tree.changelog.capacity(),
            merkle_tree.roots.capacity(),
            merkle_tree.canopy_depth,
            indexed_changelog_metadata.capacity(),
        );
        if bytes.len() < expected_size {
            return Err(IndexedMerkleTreeError::ConcurrentMerkleTree(
                ConcurrentMerkleTreeError::BufferSize(expected_size, bytes.len()),
            ));
        }

        let indexed_changelog =
            unsafe { read_cyclic_bounded_vec_at(bytes, &mut offset, &indexed_changelog_metadata) };

        Ok(Self(IndexedMerkleTree {
            merkle_tree,
            indexed_changelog,
            _index: PhantomData,
        }))
    }
}

impl<H, I, const HEIGHT: usize> Deref for IndexedMerkleTreeCopy<H, I, HEIGHT>
where
    H: Hasher,
    I: CheckedAdd + CheckedSub + Copy + Clone + PartialOrd + ToBytes + TryFrom<usize> + Unsigned,
    usize: From<I>,
{
    type Target = IndexedMerkleTree<H, I, HEIGHT>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use ark_bn254::Fr;
    use ark_ff::{BigInteger, PrimeField, UniformRand};
    use light_hasher::Poseidon;
    use rand::thread_rng;

    use crate::zero_copy::IndexedMerkleTreeZeroCopyMut;

    use super::*;

    fn from_bytes_copy<
        const HEIGHT: usize,
        const CHANGELOG_SIZE: usize,
        const ROOTS: usize,
        const CANOPY_DEPTH: usize,
        const INDEXED_CHANGELOG_SIZE: usize,
        const OPERATIONS: usize,
    >() {
        let mut mt_1 = IndexedMerkleTree::<Poseidon, usize, HEIGHT>::new(
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
            IndexedMerkleTree::<Poseidon, usize, HEIGHT>::size_in_account(
                HEIGHT,
                CHANGELOG_SIZE,
                ROOTS,
                CANOPY_DEPTH,
                INDEXED_CHANGELOG_SIZE
            )
        ];

        {
            let mut mt_2 =
                IndexedMerkleTreeZeroCopyMut::<Poseidon, usize, HEIGHT>::from_bytes_zero_copy_init(
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
                IndexedMerkleTreeZeroCopyMut::<Poseidon, usize, HEIGHT>::from_bytes_zero_copy_mut(
                    &mut bytes,
                )
                .unwrap();

            let leaf: [u8; 32] = Fr::rand(&mut rng)
                .into_bigint()
                .to_bytes_be()
                .try_into()
                .unwrap();
            mt_1.append(&leaf).unwrap();
            mt_2.append(&leaf).unwrap();

            assert_eq!(mt_1, *mt_2);
        }

        // Read a copy of that Merkle tree.
        let mt_2 =
            IndexedMerkleTreeCopy::<Poseidon, usize, HEIGHT>::from_bytes_copy(&bytes).unwrap();

        assert_eq!(mt_1, *mt_2);
    }

    #[test]
    fn test_from_bytes_copy_26_1400_2400_10_256_1024() {
        const HEIGHT: usize = 26;
        const CHANGELOG_SIZE: usize = 1400;
        const ROOTS: usize = 2400;
        const CANOPY_DEPTH: usize = 10;
        const INDEXED_CHANGELOG_SIZE: usize = 256;

        const OPERATIONS: usize = 1024;

        from_bytes_copy::<
            HEIGHT,
            CHANGELOG_SIZE,
            ROOTS,
            CANOPY_DEPTH,
            INDEXED_CHANGELOG_SIZE,
            OPERATIONS,
        >()
    }
}
