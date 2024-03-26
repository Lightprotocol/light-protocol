use std::{marker::PhantomData, mem, slice};

use light_bounded_vec::{BoundedVec, CyclicBoundedVec};
pub use light_hasher;
use light_hasher::Hasher;
pub mod changelog;
pub mod errors;
pub mod hash;
use crate::{
    changelog::{ChangelogEntry, MerklePaths},
    errors::ConcurrentMerkleTreeError,
    hash::{compute_parent_node, compute_root},
};

#[repr(C)]
#[derive(Debug)]
pub struct ConcurrentMerkleTreeMetadata {
    pub height: usize,
    pub changelog_size: usize,
    pub current_changelog_index: usize,
    pub roots_size: usize,
    pub current_root_index: usize,

    pub next_index: usize,
    pub sequence_number: usize,
    pub rightmost_leaf: [u8; 32],
}

/// [Concurrent Merkle tree](https://drive.google.com/file/d/1BOpa5OFmara50fTvL0VIVYjtg-qzHCVc/view)
/// which allows for multiple requests of updating leaves, without making any
/// of the requests invalid, as long as they are not modyfing the same leaf.
///
/// When any of the above happens, some of the concurrent requests are going to
/// be invalid, forcing the clients to re-generate the Merkle proof. But that's
/// still better than having such a failure after any update happening in the
/// middle of requesting the update.
///
/// Due to ability to make a decent number of concurrent update requests to be
/// valid, no lock is necessary.
#[repr(C)]
#[derive(Debug)]
// TODO(vadorovsky): The only reason why are we still keeping `HEIGHT` as a
// const generic here is that removing it would require keeping a `BoundecVec`
// inside `CyclicBoundedVec`. Casting byte slices to such nested vector is not
// a trivial task, but we might eventually do it at some point.
pub struct ConcurrentMerkleTree<'a, H, const HEIGHT: usize>
where
    H: Hasher,
{
    pub height: usize,

    pub changelog_capacity: usize,
    pub changelog_length: usize,
    pub current_changelog_index: usize,

    pub roots_capacity: usize,
    pub roots_length: usize,
    pub current_root_index: usize,

    pub canopy_depth: usize,

    pub next_index: usize,
    pub sequence_number: usize,
    pub rightmost_leaf: [u8; 32],

    /// Hashes of subtrees.
    pub filled_subtrees: BoundedVec<'a, [u8; 32]>,
    /// History of Merkle proofs.
    pub changelog: CyclicBoundedVec<'a, ChangelogEntry<HEIGHT>>,
    /// History of roots.
    pub roots: CyclicBoundedVec<'a, [u8; 32]>,
    /// Cached upper nodes.
    pub canopy: BoundedVec<'a, [u8; 32]>,

    _hasher: PhantomData<H>,
}

pub type ConcurrentMerkleTree22<'a, H> = ConcurrentMerkleTree<'a, H, 22>;
pub type ConcurrentMerkleTree26<'a, H> = ConcurrentMerkleTree<'a, H, 26>;
pub type ConcurrentMerkleTree32<'a, H> = ConcurrentMerkleTree<'a, H, 32>;
pub type ConcurrentMerkleTree40<'a, H> = ConcurrentMerkleTree<'a, H, 40>;

impl<'a, H, const HEIGHT: usize> ConcurrentMerkleTree<'a, H, HEIGHT>
where
    H: Hasher,
{
    /// Number of nodes to include in canopy, based on `canopy_depth`.
    #[inline(always)]
    pub fn canopy_size(canopy_depth: usize) -> usize {
        (1 << (canopy_depth + 1)) - 2
    }

    pub fn new(
        height: usize,
        changelog_size: usize,
        roots_size: usize,
        canopy_depth: usize,
    ) -> Self {
        Self {
            height,

            changelog_capacity: changelog_size,
            changelog_length: 0,
            current_changelog_index: 0,

            roots_capacity: roots_size,
            roots_length: 0,
            current_root_index: 0,

            canopy_depth,

            next_index: 0,
            sequence_number: 0,
            rightmost_leaf: [0u8; 32],

            filled_subtrees: BoundedVec::with_capacity(height),
            changelog: CyclicBoundedVec::with_capacity(changelog_size),
            roots: CyclicBoundedVec::with_capacity(roots_size),
            canopy: BoundedVec::with_capacity(Self::canopy_size(canopy_depth)),

            _hasher: PhantomData,
        }
    }

    /// Creates a copy of `ConcurrentMerkleTree` from the given byte slices.
    ///
    /// * `bytes_struct` is casted directly into a reference of
    ///   `ConcurrentMerkleTree`, then the value of the each primitive field is
    ///   copied.
    /// * `bytes_filled_subtrees` is used to create a `BoundedVec` directly.
    ///   That `BoundedVec` is assigned to the struct.
    /// * `bytes_changelog` is used to create a `CyclicBoundedVec` directly.
    ///   That `CyclicBoundedVec` is assigned to the struct.
    /// * `bytes_roots` is used to create a `CyclicBoundedVec` directly. That
    ///   `CyclicBoundedVec` is assigned to the struct.
    ///
    /// # Purpose
    ///
    /// This method is meant to be used mostly in the SDK code, to convert
    /// fetched Solana accounts to actual Merkle trees. Creating a copy is the
    /// safest way of conversion in async Rust.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. This method validates only sizes of slices.
    /// Ensuring the alignment and that the slices provide actual data of the
    /// Merkle tree is the caller's responsibility.
    ///
    /// It can be used correctly in async Rust.
    pub unsafe fn copy_from_bytes(
        bytes_struct: &[u8],
        bytes_filled_subtrees: &[u8],
        bytes_changelog: &[u8],
        bytes_roots: &[u8],
    ) -> Result<Self, ConcurrentMerkleTreeError> {
        let expected_bytes_struct_size = mem::size_of::<Self>();
        if bytes_struct.len() != expected_bytes_struct_size {
            return Err(ConcurrentMerkleTreeError::StructBufferSize(
                expected_bytes_struct_size,
                bytes_struct.len(),
            ));
        }
        let struct_ref: *mut Self = bytes_struct.as_ptr() as _;

        let mut merkle_tree = unsafe {
            Self {
                height: (*struct_ref).height,

                changelog_capacity: (*struct_ref).changelog_capacity,
                changelog_length: (*struct_ref).changelog_length,
                current_changelog_index: (*struct_ref).current_changelog_index,

                roots_capacity: (*struct_ref).roots_capacity,
                roots_length: (*struct_ref).roots_length,
                current_root_index: (*struct_ref).current_root_index,

                canopy_depth: (*struct_ref).canopy_depth,

                next_index: (*struct_ref).next_index,
                sequence_number: (*struct_ref).sequence_number,
                rightmost_leaf: (*struct_ref).rightmost_leaf,

                filled_subtrees: BoundedVec::with_capacity((*struct_ref).height),
                changelog: CyclicBoundedVec::with_capacity((*struct_ref).changelog_capacity),
                roots: CyclicBoundedVec::with_capacity((*struct_ref).roots_capacity),
                canopy: BoundedVec::with_capacity(Self::canopy_size((*struct_ref).canopy_depth)),

                _hasher: PhantomData,
            }
        };

        let expected_bytes_filled_subtrees_size = mem::size_of::<[u8; 32]>() * (*struct_ref).height;
        if bytes_filled_subtrees.len() != expected_bytes_filled_subtrees_size {
            return Err(ConcurrentMerkleTreeError::FilledSubtreesBufferSize(
                expected_bytes_filled_subtrees_size,
                bytes_filled_subtrees.len(),
            ));
        }
        let filled_subtrees: &[[u8; 32]] = slice::from_raw_parts(
            bytes_filled_subtrees.as_ptr() as *const _,
            (*struct_ref).height,
        );
        for subtree in filled_subtrees.iter() {
            merkle_tree.filled_subtrees.push(*subtree)?;
        }

        let expected_bytes_changelog_size =
            mem::size_of::<ChangelogEntry<HEIGHT>>() * (*struct_ref).changelog_capacity;
        if bytes_changelog.len() != expected_bytes_changelog_size {
            return Err(ConcurrentMerkleTreeError::ChangelogBufferSize(
                expected_bytes_changelog_size,
                bytes_changelog.len(),
            ));
        }
        let changelog: &[ChangelogEntry<HEIGHT>] = slice::from_raw_parts(
            bytes_changelog.as_ptr() as *const _,
            (*struct_ref).changelog_length,
        );
        for changelog_entry in changelog.iter() {
            merkle_tree.changelog.push(changelog_entry.clone())?;
        }

        let expected_bytes_roots_size = mem::size_of::<[u8; 32]>() * (*struct_ref).roots_capacity;
        if bytes_roots.len() != expected_bytes_roots_size {
            return Err(ConcurrentMerkleTreeError::RootBufferSize(
                expected_bytes_roots_size,
                bytes_roots.len(),
            ));
        }
        let roots: &[[u8; 32]] =
            slice::from_raw_parts(bytes_roots.as_ptr() as *const _, (*struct_ref).roots_length);
        for root in roots.iter() {
            merkle_tree.roots.push(*root)?;
        }

        Ok(merkle_tree)
    }

    /// Casts a byte slice into `ConcurrentMerkleTree`.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. Ensuring the size and alignment of the byte
    /// slice is the caller's responsibility.
    pub unsafe fn struct_from_bytes(
        bytes_struct: &'a [u8],
    ) -> Result<&'a Self, ConcurrentMerkleTreeError> {
        let expected_bytes_struct_size = mem::size_of::<Self>();
        if bytes_struct.len() != expected_bytes_struct_size {
            return Err(ConcurrentMerkleTreeError::StructBufferSize(
                expected_bytes_struct_size,
                bytes_struct.len(),
            ));
        }
        let tree: *const Self = bytes_struct.as_ptr() as _;
        Ok(&*tree)
    }

    /// Casts a byte slice into `ConcurrentMerkleTree`.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. Ensuring the size and alignment of the byte
    /// slice is the caller's responsibility.
    pub unsafe fn struct_from_bytes_mut(
        bytes_struct: &[u8],
    ) -> Result<&mut Self, ConcurrentMerkleTreeError> {
        let expected_bytes_struct_size = mem::size_of::<Self>();
        if bytes_struct.len() != expected_bytes_struct_size {
            return Err(ConcurrentMerkleTreeError::StructBufferSize(
                expected_bytes_struct_size,
                bytes_struct.len(),
            ));
        }
        let tree: *mut Self = bytes_struct.as_ptr() as _;
        Ok(&mut *tree)
    }

    /// Casts a byte slice into a `CyclicBoundedVec` containing MErkle tree
    /// roots.
    ///
    /// # Purpose
    ///
    /// This method is meant to be used mostly in Solana programs, where memory
    /// constraints are tight and we want to make sure no data is copied.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. This method validates only sizes of slices.
    /// Ensuring the alignment and that the slices provide actual data of the
    /// Merkle tree is the caller's responsibility.
    ///
    /// Calling it in async context (or anywhere where the underlying data can
    /// be moved in the memory) is certainly going to cause undefined behavior.
    pub unsafe fn roots_from_bytes(
        bytes_roots: &[u8],
        next_index: usize,
        length: usize,
        capacity: usize,
    ) -> Result<CyclicBoundedVec<[u8; 32]>, ConcurrentMerkleTreeError> {
        let expected_bytes_roots_size = mem::size_of::<[u8; 32]>() * capacity;
        if bytes_roots.len() != expected_bytes_roots_size {
            return Err(ConcurrentMerkleTreeError::RootBufferSize(
                expected_bytes_roots_size,
                bytes_roots.len(),
            ));
        }
        Ok(CyclicBoundedVec::from_raw_parts(
            bytes_roots.as_ptr() as _,
            next_index,
            length,
            capacity,
        ))
    }

    /// Casts byte slices into `ConcurrentMerkleTree`.
    ///
    /// * `bytes_struct` is casted directly into a reference of
    ///   `ConcurrentMerkleTree`.
    /// * `bytes_filled_subtrees` is used to create a `BoundedVec` directly.
    ///   That `BoundedVec` is assigned to the struct.
    /// * `bytes_changelog` is used to create a `CyclicBoundedVec` directly.
    ///   That `CyclicBoundedVec` is assigned to the struct.
    /// * `bytes_roots` is used to create a `CyclicBoundedVec` directly. That
    ///   `CyclicBoundedVec` is assigned to the struct.
    ///
    /// # Purpose
    ///
    /// This method is meant to be used mostly in Solana programs, where memory
    /// constraints are tight and we want to make sure no data is copied.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. This method validates only sizes of slices.
    /// Ensuring the alignment and that the slices provide actual data of the
    /// Merkle tree is the caller's responsibility.
    ///
    /// Calling it in async context (or anywhere where the underlying data can
    /// be moved in the memory) is certainly going to cause undefined behavior.
    pub unsafe fn from_bytes(
        bytes_struct: &'a [u8],
        bytes_filled_subtrees: &'a [u8],
        bytes_changelog: &'a [u8],
        bytes_roots: &'a [u8],
        bytes_canopy: &'a [u8],
    ) -> Result<&'a Self, ConcurrentMerkleTreeError> {
        let expected_bytes_struct_size = mem::size_of::<Self>();
        if bytes_struct.len() != expected_bytes_struct_size {
            return Err(ConcurrentMerkleTreeError::StructBufferSize(
                expected_bytes_struct_size,
                bytes_struct.len(),
            ));
        }
        let tree = Self::struct_from_bytes_mut(bytes_struct)?;

        // Restore the vectors correctly, by pointing them to the appropriate
        // byte slices as underlying data. The most unsafe part of this code.
        // Here be dragons!
        let expected_bytes_filled_subtrees_size = mem::size_of::<[u8; 32]>() * tree.height;
        if bytes_filled_subtrees.len() != expected_bytes_filled_subtrees_size {
            return Err(ConcurrentMerkleTreeError::FilledSubtreesBufferSize(
                expected_bytes_filled_subtrees_size,
                bytes_filled_subtrees.len(),
            ));
        }
        tree.filled_subtrees = BoundedVec::from_raw_parts(
            bytes_filled_subtrees.as_ptr() as _,
            tree.height,
            tree.height,
        );

        let expected_bytes_changelog_size =
            mem::size_of::<ChangelogEntry<HEIGHT>>() * tree.changelog_capacity;
        if bytes_changelog.len() != expected_bytes_changelog_size {
            return Err(ConcurrentMerkleTreeError::ChangelogBufferSize(
                expected_bytes_changelog_size,
                bytes_changelog.len(),
            ));
        }
        tree.changelog = CyclicBoundedVec::from_raw_parts(
            bytes_changelog.as_ptr() as _,
            tree.current_changelog_index + 1,
            tree.changelog_length,
            tree.changelog_capacity,
        );

        let expected_bytes_roots_size = mem::size_of::<[u8; 32]>() * tree.roots_capacity;
        if bytes_roots.len() != expected_bytes_roots_size {
            return Err(ConcurrentMerkleTreeError::RootBufferSize(
                expected_bytes_roots_size,
                bytes_roots.len(),
            ));
        }
        tree.roots = Self::roots_from_bytes(
            bytes_roots,
            tree.current_root_index + 1,
            tree.roots_length,
            tree.roots_capacity,
        )?;

        let canopy_size = Self::canopy_size(tree.canopy_depth);
        let expected_canopy_size = mem::size_of::<[u8; 32]>() * canopy_size;
        if bytes_canopy.len() != expected_canopy_size {
            return Err(ConcurrentMerkleTreeError::CanopyBufferSize(
                expected_canopy_size,
                bytes_canopy.len(),
            ));
        }
        tree.canopy =
            BoundedVec::from_raw_parts(bytes_canopy.as_ptr() as _, canopy_size, canopy_size);

        Ok(tree)
    }

    /// Assigns byte slices into vectors belonging to `ConcurrentMerkleTree`.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. Ensuring the size and alignment of the byte
    /// slices is the caller's responsibility.
    #[allow(clippy::too_many_arguments)]
    unsafe fn fill_vectors_mut<'b>(
        &'b mut self,
        bytes_filled_subtrees: &'b mut [u8],
        bytes_changelog: &'b mut [u8],
        bytes_roots: &'b mut [u8],
        bytes_canopy: &'b mut [u8],
        subtrees_length: usize,
        changelog_next_index: usize,
        changelog_length: usize,
        roots_next_index: usize,
        roots_length: usize,
        canopy_length: usize,
    ) -> Result<(), ConcurrentMerkleTreeError> {
        // Restore the vectors correctly, by pointing them to the appropriate
        // byte slices as underlying data. The most unsafe part of this code.
        // Here be dragons!
        let expected_bytes_filled_subtrees_size = mem::size_of::<[u8; 32]>() * self.height;
        if bytes_filled_subtrees.len() != expected_bytes_filled_subtrees_size {
            return Err(ConcurrentMerkleTreeError::FilledSubtreesBufferSize(
                expected_bytes_filled_subtrees_size,
                bytes_filled_subtrees.len(),
            ));
        }
        self.filled_subtrees = BoundedVec::from_raw_parts(
            bytes_filled_subtrees.as_mut_ptr() as _,
            subtrees_length,
            self.height,
        );

        let expected_bytes_changelog_size =
            mem::size_of::<ChangelogEntry<HEIGHT>>() * self.changelog_capacity;
        if bytes_changelog.len() != expected_bytes_changelog_size {
            return Err(ConcurrentMerkleTreeError::ChangelogBufferSize(
                expected_bytes_changelog_size,
                bytes_changelog.len(),
            ));
        }
        self.changelog = CyclicBoundedVec::from_raw_parts(
            bytes_changelog.as_mut_ptr() as _,
            changelog_next_index,
            changelog_length,
            self.changelog_capacity,
        );

        let expected_bytes_roots_size = mem::size_of::<[u8; 32]>() * self.roots_capacity;
        if bytes_roots.len() != expected_bytes_roots_size {
            return Err(ConcurrentMerkleTreeError::RootBufferSize(
                expected_bytes_roots_size,
                bytes_roots.len(),
            ));
        }
        self.roots = CyclicBoundedVec::from_raw_parts(
            bytes_roots.as_mut_ptr() as _,
            roots_next_index,
            roots_length,
            self.roots_capacity,
        );

        let canopy_size = Self::canopy_size(self.canopy_depth);
        let expected_canopy_size = mem::size_of::<[u8; 32]>() * canopy_size;
        if bytes_canopy.len() != expected_canopy_size {
            return Err(ConcurrentMerkleTreeError::CanopyBufferSize(
                expected_canopy_size,
                bytes_canopy.len(),
            ));
        }
        self.canopy =
            BoundedVec::from_raw_parts(bytes_canopy.as_mut_ptr() as _, canopy_length, canopy_size);

        Ok(())
    }

    /// Casts byte slices into `ConcurrentMerkleTree`.
    ///
    /// * `bytes_struct` is casted directly into a reference of
    ///   `ConcurrentMerkleTree`.
    /// * `bytes_filled_subtrees` is used to create a `BoundedVec` directly.
    ///   That `BoundedVec` is assigned to the struct.
    /// * `bytes_changelog` is used to create a `CyclicBoundedVec` directly.
    ///   That `CyclicBoundedVec` is assigned to the struct.
    /// * `bytes_roots` is used to create a `CyclicBoundedVec` directly. That
    ///   `CyclicBoundedVec` is assigned to the struct.
    ///
    /// # Purpose
    ///
    /// This method is meant to be used mostly in Solana programs, where memory
    /// constraints are tight and we want to make sure no data is copied.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. This method validates only sizes of slices.
    /// Ensuring the alignment and that the slices provide actual data of the
    /// Merkle tree is the caller's responsibility.
    ///
    /// Calling it in async context (or anywhere where the underlying data can
    /// be moved in the memory) is certainly going to cause undefined behavior.
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
        let tree = ConcurrentMerkleTree::struct_from_bytes_mut(bytes_struct)?;

        tree.height = height;

        tree.changelog_capacity = changelog_size;
        tree.changelog_length = 0;
        tree.current_changelog_index = 0;

        tree.roots_capacity = roots_size;
        tree.roots_length = 0;
        tree.current_root_index = 0;

        tree.canopy_depth = canopy_depth;

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
        )?;
        Ok(tree)
    }

    /// Casts byte slices into `ConcurrentMerkleTree`.
    ///
    /// * `bytes_struct` is casted directly into a reference of
    ///   `ConcurrentMerkleTree`.
    /// * `bytes_filled_subtrees` is used to create a `BoundedVec` directly.
    ///   That `BoundedVec` is assigned to the struct.
    /// * `bytes_changelog` is used to create a `CyclicBoundedVec` directly.
    ///   That `CyclicBoundedVec` is assigned to the struct.
    /// * `bytes_roots` is used to create a `CyclicBoundedVec` directly. That
    ///   `CyclicBoundedVec` is assigned to the struct.
    ///
    /// # Purpose
    ///
    /// This method is meant to be used mostly in Solana programs, where memory
    /// constraints are tight and we want to make sure no data is copied.
    ///
    /// # Safety
    ///
    /// This is highly unsafe. This method validates only sizes of slices.
    /// Ensuring the alignment and that the slices provide actual data of the
    /// Merkle tree is the caller's responsibility.
    ///
    /// Calling it in async context (or anywhere where the underlying data can
    /// be moved in the memory) is certainly going to cause undefined behavior.
    pub unsafe fn from_bytes_mut<'b>(
        bytes_struct: &'b mut [u8],
        bytes_filled_subtrees: &'b mut [u8],
        bytes_changelog: &'b mut [u8],
        bytes_roots: &'b mut [u8],
        bytes_canopy: &'b mut [u8],
    ) -> Result<&'b mut Self, ConcurrentMerkleTreeError> {
        let tree = ConcurrentMerkleTree::struct_from_bytes_mut(bytes_struct)?;
        tree.fill_vectors_mut(
            bytes_filled_subtrees,
            bytes_changelog,
            bytes_roots,
            bytes_canopy,
            tree.height,
            tree.current_changelog_index + 1,
            tree.changelog_length,
            tree.current_root_index + 1,
            tree.roots_length,
            Self::canopy_size(tree.canopy_depth),
        )?;
        Ok(tree)
    }

    /// Initializes the Merkle tree.
    pub fn init(&mut self) -> Result<(), ConcurrentMerkleTreeError> {
        // Initialize root.
        let root = H::zero_bytes()[self.height];
        self.roots.push(root)?;
        self.roots_length += 1;

        // Initialize changelog.
        if self.changelog_capacity > 0 {
            let path = std::array::from_fn(|i| H::zero_bytes()[i]);
            let changelog_entry = ChangelogEntry {
                root,
                path,
                index: 0,
            };
            self.changelog.push(changelog_entry)?;
            self.changelog_length += 1;
        }

        // Initialize filled subtrees.
        for i in 0..self.height {
            self.filled_subtrees.push(H::zero_bytes()[i]).unwrap();
        }

        // Initialize canopy.
        for level_i in 0..self.canopy_depth {
            let level_nodes = 1 << (level_i + 1);
            for _ in 0..level_nodes {
                let node = H::zero_bytes()[self.height - level_i - 1];
                self.canopy.push(node)?;
            }
        }

        Ok(())
    }

    /// Increments the changelog counter. If it reaches the limit, it starts
    /// from the beginning.
    fn inc_current_changelog_index(&mut self) -> Result<(), ConcurrentMerkleTreeError> {
        if self.changelog_capacity > 0 {
            if self.changelog_length < self.changelog_capacity {
                self.changelog_length = self
                    .changelog_length
                    .checked_add(1)
                    .ok_or(ConcurrentMerkleTreeError::IntegerOverflow)?;
            }
            self.current_changelog_index =
                (self.current_changelog_index + 1) % self.changelog_capacity;
        }
        Ok(())
    }

    /// Increments the root counter. If it reaches the limit, it starts from
    /// the beginning.
    fn inc_current_root_index(&mut self) -> Result<(), ConcurrentMerkleTreeError> {
        if self.roots_length < self.roots_capacity {
            self.roots_length = self
                .roots_length
                .checked_add(1)
                .ok_or(ConcurrentMerkleTreeError::IntegerOverflow)?;
        }
        self.current_root_index = (self.current_root_index + 1) % self.roots_capacity;
        Ok(())
    }

    /// Returns the index of the current changelog entry.
    pub fn changelog_index(&self) -> usize {
        self.current_changelog_index
    }

    /// Returns the index of the current root in the tree's root buffer.
    pub fn root_index(&self) -> usize {
        self.current_root_index
    }

    /// Returns the current root.
    pub fn root(&self) -> Result<[u8; 32], ConcurrentMerkleTreeError> {
        self.roots
            .get(self.root_index())
            .ok_or(ConcurrentMerkleTreeError::RootHigherThanMax)
            .copied()
    }

    pub fn current_index(&self) -> usize {
        let next_index = self.next_index();
        if next_index > 0 {
            next_index - 1
        } else {
            next_index
        }
    }

    pub fn next_index(&self) -> usize {
        self.next_index
    }

    pub fn update_proof_from_canopy(
        &self,
        leaf_index: usize,
        proof: &mut BoundedVec<[u8; 32]>,
    ) -> Result<(), ConcurrentMerkleTreeError> {
        let mut node_index = ((1 << self.height) + leaf_index) >> (self.height - self.canopy_depth);
        while node_index > 1 {
            // `node_index - 2` maps to the canopy index.
            let canopy_index = node_index - 2;
            let canopy_index = if canopy_index % 2 == 0 {
                canopy_index + 1
            } else {
                canopy_index - 1
            };
            proof.push(self.canopy[canopy_index])?;
            node_index >>= 1;
        }

        Ok(())
    }

    /// Returns an updated Merkle proof.
    ///
    /// The update is performed by checking whether there are any new changelog
    /// entries and whether they contain changes which affect the current
    /// proof. To be precise, for each changelog entry, it's done in the
    /// following steps:
    ///
    /// * Check if the changelog entry was directly updating the `leaf_index`
    ///   we are trying to update.
    ///   * If no (we check that condition first, since it's more likely),
    ///     it means that there is a change affecting the proof, but not the
    ///     leaf.
    ///     Check which element from our proof was affected by the change
    ///     (using the `critbit_index` method) and update it (copy the new
    ///     element from the changelog to our updated proof).
    ///   * If yes, it means that the same leaf we want to update was already
    ///     updated. In such case, updating the proof is not possible.
    fn update_proof_from_changelog(
        &self,
        changelog_index: usize,
        leaf_index: usize,
        proof: &mut BoundedVec<[u8; 32]>,
    ) -> Result<(), ConcurrentMerkleTreeError> {
        let mut i = changelog_index + 1;
        while i != self.changelog_index() + 1 {
            self.changelog[i].update_proof(leaf_index, proof)?;
            i = (i + 1) % self.changelog_length;
        }

        Ok(())
    }

    /// Checks whether the given Merkle `proof` for the given `node` (with index
    /// `i`) is valid. The proof is valid when computing parent node hashes using
    /// the whole path of the proof gives the same result as the given `root`.
    pub fn validate_proof(
        &self,
        leaf: &[u8; 32],
        leaf_index: usize,
        proof: &BoundedVec<[u8; 32]>,
    ) -> Result<(), ConcurrentMerkleTreeError> {
        let expected_root = self.root()?;
        let computed_root = compute_root::<H>(leaf, leaf_index, proof)?;
        // with the following print set the expected and computed roots are the same
        // comment the statment to reproduce the error with test programs/account-compression/tests/merkle_tree_tests.rs
        // in accounts-compression run cargo test-sbf test_nullify_leaves
        #[cfg(target_os = "solana")]
        solana_program::msg!("leaf: {:?}", leaf);
        if computed_root == expected_root {
            Ok(())
        } else {
            Err(ConcurrentMerkleTreeError::InvalidProof(
                expected_root,
                computed_root,
            ))
        }
    }

    /// Updates the leaf under `leaf_index` with the `new_leaf` value.
    ///
    /// 1. Computes the new path and root from `new_leaf` and Merkle proof
    ///    (`proof`).
    /// 2. Stores the new path as the latest changelog entry and increments the
    ///    latest changelog index.
    /// 3. Stores the latest root and increments the latest root index.
    /// 4. If new leaf is at the rightmost index, stores it as the new
    ///    rightmost leaft and stores the Merkle proof as the new rightmost
    ///    proof.
    ///
    /// # Validation
    ///
    /// This method doesn't validate the proof. Caller is responsible for
    /// doing that before.
    fn update_leaf_in_tree(
        &mut self,
        new_leaf: &[u8; 32],
        leaf_index: usize,
        proof: &BoundedVec<[u8; 32]>,
    ) -> Result<ChangelogEntry<HEIGHT>, ConcurrentMerkleTreeError> {
        let mut node = *new_leaf;
        let mut changelog_path = [[0u8; 32]; HEIGHT];

        for (j, sibling) in proof.iter().enumerate() {
            changelog_path[j] = node;
            node = compute_parent_node::<H>(&node, sibling, leaf_index, j)?;
        }

        let changelog_entry = ChangelogEntry::new(node, changelog_path, leaf_index);
        if self.changelog_capacity > 0 {
            self.inc_current_changelog_index()?;
            self.changelog.push(changelog_entry.clone())?;
        }

        self.inc_current_root_index()?;
        self.roots.push(node)?;

        changelog_entry.update_subtrees(self.next_index - 1, &mut self.filled_subtrees);

        // Check if we updated the rightmost leaf.
        if self.next_index() < (1 << self.height) && leaf_index >= self.current_index() {
            self.rightmost_leaf = *new_leaf;
        }

        Ok(changelog_entry)
    }

    /// Replaces the `old_leaf` under the `leaf_index` with a `new_leaf`, using
    /// the given `proof` and `changelog_index` (pointing to the changelog entry
    /// which was the newest at the time of preparing the proof).
    #[inline(never)]
    pub fn update(
        &mut self,
        changelog_index: usize,
        old_leaf: &[u8; 32],
        new_leaf: &[u8; 32],
        leaf_index: usize,
        proof: &mut BoundedVec<[u8; 32]>,
    ) -> Result<ChangelogEntry<HEIGHT>, ConcurrentMerkleTreeError> {
        let expected_proof_len = self.height - self.canopy_depth;
        if proof.len() != expected_proof_len {
            return Err(ConcurrentMerkleTreeError::InvalidProofLength(
                expected_proof_len,
                proof.len(),
            ));
        }
        if leaf_index >= self.next_index() {
            return Err(ConcurrentMerkleTreeError::CannotUpdateEmpty);
        }

        if self.canopy_depth > 0 {
            self.update_proof_from_canopy(leaf_index, proof)?;
        }
        if self.changelog_capacity > 0 {
            self.update_proof_from_changelog(changelog_index, leaf_index, proof)?;
        }
        self.validate_proof(old_leaf, leaf_index, proof)?;
        self.update_leaf_in_tree(new_leaf, leaf_index, proof)
    }

    /// Appends a new leaf to the tree.
    pub fn append(
        &mut self,
        leaf: &[u8; 32],
    ) -> Result<ChangelogEntry<HEIGHT>, ConcurrentMerkleTreeError> {
        let changelog_entries = self.append_batch(&[leaf])?;
        let changelog_entry = changelog_entries
            .first()
            .ok_or(ConcurrentMerkleTreeError::EmptyChangelogEntries)?
            .to_owned();
        Ok(changelog_entry)
    }

    /// Appends a batch of new leaves to the tree.
    pub fn append_batch(
        &mut self,
        leaves: &[&[u8; 32]],
    ) -> Result<Vec<ChangelogEntry<HEIGHT>>, ConcurrentMerkleTreeError> {
        if (self.next_index + leaves.len() - 1) >= 1 << self.height {
            return Err(ConcurrentMerkleTreeError::TreeFull);
        }

        let first_leaf_index = self.next_index;
        // Buffer of Merkle paths.
        let mut merkle_paths = MerklePaths::<H>::new(self.height, leaves.len());

        for (leaf_i, leaf) in leaves.iter().enumerate() {
            let mut current_index = self.next_index;
            let mut current_node = leaf.to_owned().to_owned();

            merkle_paths.add_leaf();

            // Limit until which we fill up the current Merkle path.
            let fillup_index = if leaf_i < (leaves.len() - 1) {
                self.next_index.trailing_ones() as usize + 1
            } else {
                self.height
            };

            // Assign the leaf to the path.
            merkle_paths.set(0, current_node);

            for i in 0..fillup_index {
                let is_left = current_index % 2 == 0;

                current_node = if is_left {
                    let empty_node = H::zero_bytes()[i];
                    self.filled_subtrees[i] = current_node;
                    H::hashv(&[&current_node, &empty_node])?
                } else {
                    H::hashv(&[&self.filled_subtrees[i], &current_node])?
                };

                if i < self.height - 1 {
                    merkle_paths.set(i + 1, current_node);
                }

                current_index /= 2;
            }

            merkle_paths.set_root(current_node);

            self.inc_current_root_index()?;
            self.roots.push(current_node)?;

            self.sequence_number = self
                .sequence_number
                .checked_add(1)
                .ok_or(ConcurrentMerkleTreeError::IntegerOverflow)?;
            self.next_index = self
                .next_index
                .checked_add(1)
                .ok_or(ConcurrentMerkleTreeError::IntegerOverflow)?;
            self.rightmost_leaf = leaf.to_owned().to_owned();
        }

        let changelog_entries = merkle_paths.to_changelog_entries(first_leaf_index)?;

        // Save changelog entries.
        if self.changelog_capacity > 0 {
            for changelog_entry in changelog_entries.iter() {
                self.inc_current_changelog_index()?;
                self.changelog.push(changelog_entry.clone())?;
            }
        }

        if self.canopy_depth > 0 {
            self.update_canopy(&changelog_entries)?;
        }

        Ok(changelog_entries)
    }

    fn update_canopy(
        &mut self,
        changelog_entries: &Vec<ChangelogEntry<HEIGHT>>,
    ) -> Result<(), ConcurrentMerkleTreeError> {
        for changelog_entry in changelog_entries {
            for (i, path_node) in changelog_entry
                .path
                .iter()
                .rev()
                .take(self.canopy_depth)
                .enumerate()
            {
                let level = self.height - i - 1;
                let index = (1 << (self.height - level)) + (changelog_entry.index >> level);
                // `index - 2` maps to the canopy index.
                self.canopy[(index - 2) as usize] = *path_node;
            }
        }

        Ok(())
    }
}
