use std::ops::Deref;

use light_bounded_vec::{BoundedVecMetadata, CyclicBoundedVecMetadata};
use light_hasher::Hasher;
use memoffset::{offset_of, span_of};

use crate::{
    errors::ConcurrentMerkleTreeError,
    offset::copy::{read_bounded_vec_at, read_cyclic_bounded_vec_at, read_value_at},
    ConcurrentMerkleTree,
};

#[derive(Debug)]
pub struct ConcurrentMerkleTreeCopy<H, const HEIGHT: usize>(ConcurrentMerkleTree<H, HEIGHT>)
where
    H: Hasher;

impl<H, const HEIGHT: usize> ConcurrentMerkleTreeCopy<H, HEIGHT>
where
    H: Hasher,
{
    pub fn struct_from_bytes_copy(
        bytes: &[u8],
    ) -> Result<(ConcurrentMerkleTree<H, HEIGHT>, usize), ConcurrentMerkleTreeError> {
        let expected_size = ConcurrentMerkleTree::<H, HEIGHT>::non_dyn_fields_size();
        if bytes.len() < expected_size {
            return Err(ConcurrentMerkleTreeError::BufferSize(
                expected_size,
                bytes.len(),
            ));
        }

        let height = usize::from_le_bytes(
            bytes[span_of!(ConcurrentMerkleTree<H, HEIGHT>, height)]
                .try_into()
                .unwrap(),
        );
        let canopy_depth = usize::from_le_bytes(
            bytes[span_of!(ConcurrentMerkleTree<H, HEIGHT>, canopy_depth)]
                .try_into()
                .unwrap(),
        );

        let mut offset = offset_of!(ConcurrentMerkleTree<H, HEIGHT>, next_index);

        let next_index = unsafe { read_value_at(bytes, &mut offset) };
        let sequence_number = unsafe { read_value_at(bytes, &mut offset) };
        let rightmost_leaf = unsafe { read_value_at(bytes, &mut offset) };
        let filled_subtrees_metadata: BoundedVecMetadata =
            unsafe { read_value_at(bytes, &mut offset) };
        let changelog_metadata: CyclicBoundedVecMetadata =
            unsafe { read_value_at(bytes, &mut offset) };
        let roots_metadata: CyclicBoundedVecMetadata = unsafe { read_value_at(bytes, &mut offset) };
        let canopy_metadata: BoundedVecMetadata = unsafe { read_value_at(bytes, &mut offset) };

        let expected_size = ConcurrentMerkleTree::<H, HEIGHT>::size_in_account(
            height,
            changelog_metadata.capacity(),
            roots_metadata.capacity(),
            canopy_depth,
        );
        if bytes.len() < expected_size {
            return Err(ConcurrentMerkleTreeError::BufferSize(
                expected_size,
                bytes.len(),
            ));
        }

        let filled_subtrees =
            unsafe { read_bounded_vec_at(bytes, &mut offset, &filled_subtrees_metadata) };
        let changelog =
            unsafe { read_cyclic_bounded_vec_at(bytes, &mut offset, &changelog_metadata) };
        let roots = unsafe { read_cyclic_bounded_vec_at(bytes, &mut offset, &roots_metadata) };
        let canopy = unsafe { read_bounded_vec_at(bytes, &mut offset, &canopy_metadata) };

        let mut merkle_tree = ConcurrentMerkleTree::new(
            height,
            changelog_metadata.capacity(),
            roots_metadata.capacity(),
            canopy_depth,
        )?;
        // SAFETY: Tree is initialized.
        unsafe {
            *merkle_tree.next_index = next_index;
            *merkle_tree.sequence_number = sequence_number;
            *merkle_tree.rightmost_leaf = rightmost_leaf;
        }
        merkle_tree.filled_subtrees = filled_subtrees;
        merkle_tree.changelog = changelog;
        merkle_tree.roots = roots;
        merkle_tree.canopy = canopy;

        Ok((merkle_tree, offset))
    }

    pub fn from_bytes_copy(bytes: &[u8]) -> Result<Self, ConcurrentMerkleTreeError> {
        let (merkle_tree, _) = Self::struct_from_bytes_copy(bytes)?;
        merkle_tree.check_size_constraints()?;
        Ok(Self(merkle_tree))
    }
}

impl<H, const HEIGHT: usize> Deref for ConcurrentMerkleTreeCopy<H, HEIGHT>
where
    H: Hasher,
{
    type Target = ConcurrentMerkleTree<H, HEIGHT>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use ark_bn254::Fr;
    use ark_ff::{BigInteger, PrimeField, UniformRand};
    use light_hasher::Poseidon;
    use rand::{thread_rng, Rng};

    use super::*;
    use crate::zero_copy::ConcurrentMerkleTreeZeroCopyMut;

    fn from_bytes_copy<
        const HEIGHT: usize,
        const CHANGELOG: usize,
        const ROOTS: usize,
        const CANOPY_DEPTH: usize,
        const OPERATIONS: usize,
    >() {
        let mut mt_1 =
            ConcurrentMerkleTree::<Poseidon, HEIGHT>::new(HEIGHT, CHANGELOG, ROOTS, CANOPY_DEPTH)
                .unwrap();
        mt_1.init().unwrap();

        // Create a buffer with random bytes - the `*_init` method should
        // initialize the buffer gracefully and the randomness shouldn't cause
        // undefined behavior.
        let mut bytes = vec![
            0u8;
            ConcurrentMerkleTree::<Poseidon, HEIGHT>::size_in_account(
                HEIGHT,
                CHANGELOG,
                ROOTS,
                CANOPY_DEPTH
            )
        ];
        thread_rng().fill(bytes.as_mut_slice());

        // Initialize a Merkle tree on top of a byte slice.
        {
            let mut mt =
                ConcurrentMerkleTreeZeroCopyMut::<Poseidon, HEIGHT>::from_bytes_zero_copy_init(
                    bytes.as_mut_slice(),
                    HEIGHT,
                    CANOPY_DEPTH,
                    CHANGELOG,
                    ROOTS,
                )
                .unwrap();
            mt.init().unwrap();

            // Ensure that it was properly initialized.
            assert_eq!(mt.height, HEIGHT);
            assert_eq!(mt.canopy_depth, CANOPY_DEPTH);
            assert_eq!(mt.next_index(), 0);
            assert_eq!(mt.sequence_number(), 0);
            assert_eq!(mt.rightmost_leaf(), Poseidon::zero_bytes()[0]);

            assert_eq!(mt.filled_subtrees.capacity(), HEIGHT);
            assert_eq!(mt.filled_subtrees.len(), HEIGHT);

            assert_eq!(mt.changelog.capacity(), CHANGELOG);
            assert_eq!(mt.changelog.len(), 1);

            assert_eq!(mt.roots.capacity(), ROOTS);
            assert_eq!(mt.roots.len(), 1);

            assert_eq!(
                mt.canopy.capacity(),
                ConcurrentMerkleTree::<Poseidon, HEIGHT>::canopy_size(CANOPY_DEPTH)
            );

            assert_eq!(mt.root(), Poseidon::zero_bytes()[HEIGHT]);
        }

        let mut rng = thread_rng();

        for _ in 0..OPERATIONS {
            // Reload the tree from bytes on each iteration.
            let mut mt_2 =
                ConcurrentMerkleTreeZeroCopyMut::<Poseidon, HEIGHT>::from_bytes_zero_copy_mut(
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
        let mt_2 = ConcurrentMerkleTreeCopy::<Poseidon, HEIGHT>::from_bytes_copy(&bytes).unwrap();

        assert_eq!(mt_1.height, mt_2.height);
        assert_eq!(mt_1.canopy_depth, mt_2.canopy_depth);
        assert_eq!(mt_1.next_index(), mt_2.next_index());
        assert_eq!(mt_1.sequence_number(), mt_2.sequence_number());
        assert_eq!(mt_1.rightmost_leaf(), mt_2.rightmost_leaf());
        assert_eq!(
            mt_1.filled_subtrees.as_slice(),
            mt_2.filled_subtrees.as_slice()
        );
    }

    #[test]
    fn test_from_bytes_copy_26_1400_2400_10_256_1024() {
        const HEIGHT: usize = 26;
        const CHANGELOG_SIZE: usize = 1400;
        const ROOTS: usize = 2400;
        const CANOPY_DEPTH: usize = 10;

        const OPERATIONS: usize = 1024;

        from_bytes_copy::<HEIGHT, CHANGELOG_SIZE, ROOTS, CANOPY_DEPTH, OPERATIONS>()
    }
}
