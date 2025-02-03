use std::{
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
};

use light_bounded_vec::{
    BoundedVec, BoundedVecMetadata, CyclicBoundedVec, CyclicBoundedVecMetadata,
};
use light_hasher::Hasher;
use memoffset::{offset_of, span_of};

use crate::{
    errors::ConcurrentMerkleTreeError,
    offset::zero_copy::{read_array_like_ptr_at, read_ptr_at, write_at},
    ConcurrentMerkleTree,
};

#[derive(Debug)]
pub struct ConcurrentMerkleTreeZeroCopy<'a, H, const HEIGHT: usize>
where
    H: Hasher,
{
    merkle_tree: mem::ManuallyDrop<ConcurrentMerkleTree<H, HEIGHT>>,
    // The purpose of this field is ensuring that the wrapper does not outlive
    // the buffer.
    _bytes: &'a [u8],
}

impl<'a, H, const HEIGHT: usize> ConcurrentMerkleTreeZeroCopy<'a, H, HEIGHT>
where
    H: Hasher,
{
    pub fn struct_from_bytes_zero_copy(
        bytes: &'a [u8],
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

        let next_index = unsafe { read_ptr_at(bytes, &mut offset) };
        let sequence_number = unsafe { read_ptr_at(bytes, &mut offset) };
        let rightmost_leaf = unsafe { read_ptr_at(bytes, &mut offset) };
        let filled_subtrees_metadata = unsafe { read_ptr_at(bytes, &mut offset) };
        let changelog_metadata: *mut CyclicBoundedVecMetadata =
            unsafe { read_ptr_at(bytes, &mut offset) };
        let roots_metadata: *mut CyclicBoundedVecMetadata =
            unsafe { read_ptr_at(bytes, &mut offset) };
        let canopy_metadata = unsafe { read_ptr_at(bytes, &mut offset) };

        let expected_size = ConcurrentMerkleTree::<H, HEIGHT>::size_in_account(
            height,
            unsafe { (*changelog_metadata).capacity() },
            unsafe { (*roots_metadata).capacity() },
            canopy_depth,
        );
        if bytes.len() < expected_size {
            return Err(ConcurrentMerkleTreeError::BufferSize(
                expected_size,
                bytes.len(),
            ));
        }

        let filled_subtrees = unsafe {
            BoundedVec::from_raw_parts(
                filled_subtrees_metadata,
                read_array_like_ptr_at(bytes, &mut offset, height),
            )
        };
        let changelog = unsafe {
            CyclicBoundedVec::from_raw_parts(
                changelog_metadata,
                read_array_like_ptr_at(bytes, &mut offset, (*changelog_metadata).capacity()),
            )
        };
        let roots = unsafe {
            CyclicBoundedVec::from_raw_parts(
                roots_metadata,
                read_array_like_ptr_at(bytes, &mut offset, (*roots_metadata).capacity()),
            )
        };
        let canopy = unsafe {
            BoundedVec::from_raw_parts(
                canopy_metadata,
                read_array_like_ptr_at(bytes, &mut offset, (*canopy_metadata).capacity()),
            )
        };

        let merkle_tree = ConcurrentMerkleTree {
            height,
            canopy_depth,
            next_index,
            sequence_number,
            rightmost_leaf,
            filled_subtrees,
            changelog,
            roots,
            canopy,
            _hasher: PhantomData,
        };
        merkle_tree.check_size_constraints()?;

        Ok((merkle_tree, offset))
    }

    pub fn from_bytes_zero_copy(bytes: &'a [u8]) -> Result<Self, ConcurrentMerkleTreeError> {
        let (merkle_tree, _) = Self::struct_from_bytes_zero_copy(bytes)?;
        merkle_tree.check_size_constraints()?;

        Ok(Self {
            merkle_tree: mem::ManuallyDrop::new(merkle_tree),
            _bytes: bytes,
        })
    }
}

impl<H, const HEIGHT: usize> Deref for ConcurrentMerkleTreeZeroCopy<'_, H, HEIGHT>
where
    H: Hasher,
{
    type Target = ConcurrentMerkleTree<H, HEIGHT>;

    fn deref(&self) -> &Self::Target {
        &self.merkle_tree
    }
}

impl<H, const HEIGHT: usize> Drop for ConcurrentMerkleTreeZeroCopy<'_, H, HEIGHT>
where
    H: Hasher,
{
    fn drop(&mut self) {
        // SAFETY: Don't do anything here! Why?
        //
        // * Primitive fields of `ConcurrentMerkleTree` implement `Copy`,
        //   therefore `drop()` has no effect on them - Rust drops them when
        //   they go out of scope.
        // * Don't drop the dynamic fields (`filled_subtrees`, `roots` etc.). In
        //   `ConcurrentMerkleTreeZeroCopy`, they are backed by buffers provided
        //   by the caller. These buffers are going to be eventually deallocated.
        //   Performing an another `drop()` here would result double `free()`
        //   which would result in aborting the program (either with `SIGABRT`
        //   or `SIGSEGV`).
    }
}

#[derive(Debug)]
pub struct ConcurrentMerkleTreeZeroCopyMut<'a, H, const HEIGHT: usize>(
    ConcurrentMerkleTreeZeroCopy<'a, H, HEIGHT>,
)
where
    H: Hasher;

impl<'a, H, const HEIGHT: usize> ConcurrentMerkleTreeZeroCopyMut<'a, H, HEIGHT>
where
    H: Hasher,
{
    pub fn from_bytes_zero_copy_mut(
        bytes: &'a mut [u8],
    ) -> Result<Self, ConcurrentMerkleTreeError> {
        Ok(Self(ConcurrentMerkleTreeZeroCopy::from_bytes_zero_copy(
            bytes,
        )?))
    }

    pub fn fill_non_dyn_fields_in_buffer(
        bytes: &mut [u8],
        height: usize,
        canopy_depth: usize,
        changelog_capacity: usize,
        roots_capacity: usize,
    ) -> Result<usize, ConcurrentMerkleTreeError> {
        let expected_size = ConcurrentMerkleTree::<H, HEIGHT>::size_in_account(
            height,
            changelog_capacity,
            roots_capacity,
            canopy_depth,
        );
        if bytes.len() < expected_size {
            return Err(ConcurrentMerkleTreeError::BufferSize(
                expected_size,
                bytes.len(),
            ));
        }

        bytes[span_of!(ConcurrentMerkleTree<H, HEIGHT>, height)]
            .copy_from_slice(&height.to_le_bytes());
        bytes[span_of!(ConcurrentMerkleTree<H, HEIGHT>, canopy_depth)]
            .copy_from_slice(&canopy_depth.to_le_bytes());

        let mut offset = offset_of!(ConcurrentMerkleTree<H, HEIGHT>, next_index);
        // next_index
        write_at::<usize>(bytes, &0_usize.to_le_bytes(), &mut offset);
        // sequence_number
        write_at::<usize>(bytes, &0_usize.to_le_bytes(), &mut offset);
        // rightmost_leaf
        write_at::<[u8; 32]>(bytes, &H::zero_bytes()[0], &mut offset);
        // filled_subtrees (metadata)
        let filled_subtrees_metadata = BoundedVecMetadata::new(height);
        write_at::<BoundedVecMetadata>(bytes, &filled_subtrees_metadata.to_le_bytes(), &mut offset);
        // changelog (metadata)
        let changelog_metadata = CyclicBoundedVecMetadata::new(changelog_capacity);
        write_at::<CyclicBoundedVecMetadata>(bytes, &changelog_metadata.to_le_bytes(), &mut offset);
        // roots (metadata)
        let roots_metadata = CyclicBoundedVecMetadata::new(roots_capacity);
        write_at::<CyclicBoundedVecMetadata>(bytes, &roots_metadata.to_le_bytes(), &mut offset);
        // canopy (metadata)
        let canopy_size = ConcurrentMerkleTree::<H, HEIGHT>::canopy_size(canopy_depth);
        let canopy_metadata = BoundedVecMetadata::new(canopy_size);
        write_at::<BoundedVecMetadata>(bytes, &canopy_metadata.to_le_bytes(), &mut offset);

        Ok(offset)
    }

    pub fn from_bytes_zero_copy_init(
        bytes: &'a mut [u8],
        height: usize,
        canopy_depth: usize,
        changelog_capacity: usize,
        roots_capacity: usize,
    ) -> Result<Self, ConcurrentMerkleTreeError> {
        Self::fill_non_dyn_fields_in_buffer(
            bytes,
            height,
            canopy_depth,
            changelog_capacity,
            roots_capacity,
        )?;
        Self::from_bytes_zero_copy_mut(bytes)
    }
}

impl<H, const HEIGHT: usize> Deref for ConcurrentMerkleTreeZeroCopyMut<'_, H, HEIGHT>
where
    H: Hasher,
{
    type Target = ConcurrentMerkleTree<H, HEIGHT>;

    fn deref(&self) -> &Self::Target {
        &self.0.merkle_tree
    }
}
impl<H, const HEIGHT: usize> DerefMut for ConcurrentMerkleTreeZeroCopyMut<'_, H, HEIGHT>
where
    H: Hasher,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.merkle_tree
    }
}

#[cfg(test)]
mod test {
    use ark_bn254::Fr;
    use ark_ff::{BigInteger, PrimeField, UniformRand};
    use light_hasher::Poseidon;
    use rand::{thread_rng, Rng};

    use super::*;

    fn load_from_bytes<
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
            assert_eq!(mt.canopy_depth, CANOPY_DEPTH,);
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
    }

    #[test]
    fn test_load_from_bytes_22_256_256_0_1024() {
        load_from_bytes::<22, 256, 256, 0, 1024>()
    }

    #[test]
    fn test_load_from_bytes_22_256_256_10_1024() {
        load_from_bytes::<22, 256, 256, 10, 1024>()
    }
}
