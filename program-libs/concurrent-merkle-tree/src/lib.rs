use std::{
    alloc::{self, handle_alloc_error, Layout},
    iter::Skip,
    marker::PhantomData,
    mem,
};

use changelog::ChangelogPath;
use light_bounded_vec::{
    BoundedVec, BoundedVecMetadata, CyclicBoundedVec, CyclicBoundedVecIterator,
    CyclicBoundedVecMetadata,
};
pub use light_hasher;
use light_hasher::Hasher;

pub mod changelog;
pub mod copy;
pub mod errors;
pub mod event;
pub mod hash;
pub mod offset;
pub mod zero_copy;

use crate::{
    changelog::ChangelogEntry,
    errors::ConcurrentMerkleTreeError,
    hash::{compute_parent_node, compute_root},
};

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
pub struct ConcurrentMerkleTree<H, const HEIGHT: usize>
where
    H: Hasher,
{
    pub height: usize,
    pub canopy_depth: usize,

    next_index: *mut usize,
    sequence_number: *mut usize,
    rightmost_leaf: *mut [u8; 32],

    /// Hashes of subtrees.
    pub filled_subtrees: BoundedVec<[u8; 32]>,
    /// History of Merkle proofs.
    pub changelog: CyclicBoundedVec<ChangelogEntry<HEIGHT>>,
    /// History of roots.
    pub roots: CyclicBoundedVec<[u8; 32]>,
    /// Cached upper nodes.
    pub canopy: BoundedVec<[u8; 32]>,

    pub _hasher: PhantomData<H>,
}

pub type ConcurrentMerkleTree26<H> = ConcurrentMerkleTree<H, 26>;

impl<H, const HEIGHT: usize> ConcurrentMerkleTree<H, HEIGHT>
where
    H: Hasher,
{
    /// Number of nodes to include in canopy, based on `canopy_depth`.
    #[inline(always)]
    pub fn canopy_size(canopy_depth: usize) -> usize {
        (1 << (canopy_depth + 1)) - 2
    }

    /// Size of the struct **without** dynamically sized fields (`BoundedVec`,
    /// `CyclicBoundedVec`).
    pub fn non_dyn_fields_size() -> usize {
        // height
        mem::size_of::<usize>()
        // changelog_capacity
        + mem::size_of::<usize>()
        // next_index
        + mem::size_of::<usize>()
        // sequence_number
        + mem::size_of::<usize>()
        // rightmost_leaf
        + mem::size_of::<[u8; 32]>()
        // filled_subtrees (metadata)
        + mem::size_of::<BoundedVecMetadata>()
        // changelog (metadata)
        + mem::size_of::<CyclicBoundedVecMetadata>()
        // roots (metadata)
        + mem::size_of::<CyclicBoundedVecMetadata>()
        // canopy (metadata)
        + mem::size_of::<BoundedVecMetadata>()
    }

    // TODO(vadorovsky): Make a macro for that.
    pub fn size_in_account(
        height: usize,
        changelog_size: usize,
        roots_size: usize,
        canopy_depth: usize,
    ) -> usize {
        // non-dynamic fields
        Self::non_dyn_fields_size()
        // filled_subtrees
        + mem::size_of::<[u8; 32]>() * height
        // changelog
        + mem::size_of::<ChangelogEntry<HEIGHT>>() * changelog_size
        // roots
        + mem::size_of::<[u8; 32]>() * roots_size
        // canopy
        + mem::size_of::<[u8; 32]>() * Self::canopy_size(canopy_depth)
    }

    fn check_size_constraints_new(
        height: usize,
        changelog_size: usize,
        roots_size: usize,
        canopy_depth: usize,
    ) -> Result<(), ConcurrentMerkleTreeError> {
        if height == 0 || HEIGHT == 0 {
            return Err(ConcurrentMerkleTreeError::HeightZero);
        }
        if height != HEIGHT {
            return Err(ConcurrentMerkleTreeError::InvalidHeight(HEIGHT));
        }
        if canopy_depth > height {
            return Err(ConcurrentMerkleTreeError::CanopyGeThanHeight);
        }
        // Changelog needs to be at least 1, because it's used for storing
        // Merkle paths in `append`/`append_batch`.
        if changelog_size == 0 {
            return Err(ConcurrentMerkleTreeError::ChangelogZero);
        }
        if roots_size == 0 {
            return Err(ConcurrentMerkleTreeError::RootsZero);
        }
        Ok(())
    }

    fn check_size_constraints(&self) -> Result<(), ConcurrentMerkleTreeError> {
        Self::check_size_constraints_new(
            self.height,
            self.changelog.capacity(),
            self.roots.capacity(),
            self.canopy_depth,
        )
    }

    pub fn new(
        height: usize,
        changelog_size: usize,
        roots_size: usize,
        canopy_depth: usize,
    ) -> Result<Self, ConcurrentMerkleTreeError> {
        Self::check_size_constraints_new(height, changelog_size, roots_size, canopy_depth)?;

        let layout = Layout::new::<usize>();
        let next_index = unsafe { alloc::alloc(layout) as *mut usize };
        if next_index.is_null() {
            handle_alloc_error(layout);
        }
        unsafe { *next_index = 0 };

        let layout = Layout::new::<usize>();
        let sequence_number = unsafe { alloc::alloc(layout) as *mut usize };
        if sequence_number.is_null() {
            handle_alloc_error(layout);
        }
        unsafe { *sequence_number = 0 };

        let layout = Layout::new::<[u8; 32]>();
        let rightmost_leaf = unsafe { alloc::alloc(layout) as *mut [u8; 32] };
        if rightmost_leaf.is_null() {
            handle_alloc_error(layout);
        }
        unsafe { *rightmost_leaf = [0u8; 32] };

        Ok(Self {
            height,
            canopy_depth,

            next_index,
            sequence_number,
            rightmost_leaf,

            filled_subtrees: BoundedVec::with_capacity(height),
            changelog: CyclicBoundedVec::with_capacity(changelog_size),
            roots: CyclicBoundedVec::with_capacity(roots_size),
            canopy: BoundedVec::with_capacity(Self::canopy_size(canopy_depth)),

            _hasher: PhantomData,
        })
    }

    /// Initializes the Merkle tree.
    pub fn init(&mut self) -> Result<(), ConcurrentMerkleTreeError> {
        self.check_size_constraints()?;

        // Initialize root.
        let root = H::zero_bytes()[self.height];
        self.roots.push(root);

        // Initialize changelog.
        let path = ChangelogPath::from_fn(|i| Some(H::zero_bytes()[i]));
        let changelog_entry = ChangelogEntry { path, index: 0 };
        self.changelog.push(changelog_entry);

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

    /// Returns the index of the current changelog entry.
    pub fn changelog_index(&self) -> usize {
        self.changelog.last_index()
    }

    /// Returns the index of the current root in the tree's root buffer.
    pub fn root_index(&self) -> usize {
        self.roots.last_index()
    }

    /// Returns the current root.
    pub fn root(&self) -> [u8; 32] {
        // PANICS: This should never happen - there is always a root in the
        // tree and `self.root_index()` should always point to an existing index.
        self.roots[self.root_index()]
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
        unsafe { *self.next_index }
    }

    pub fn inc_next_index(&mut self) -> Result<(), ConcurrentMerkleTreeError> {
        unsafe {
            *self.next_index = self
                .next_index()
                .checked_add(1)
                .ok_or(ConcurrentMerkleTreeError::IntegerOverflow)?;
        }
        Ok(())
    }

    pub fn sequence_number(&self) -> usize {
        unsafe { *self.sequence_number }
    }

    pub fn inc_sequence_number(&mut self) -> Result<(), ConcurrentMerkleTreeError> {
        unsafe {
            *self.sequence_number = self
                .sequence_number()
                .checked_add(1)
                .ok_or(ConcurrentMerkleTreeError::IntegerOverflow)?;
        }
        Ok(())
    }

    pub fn rightmost_leaf(&self) -> [u8; 32] {
        unsafe { *self.rightmost_leaf }
    }

    fn set_rightmost_leaf(&mut self, leaf: &[u8; 32]) {
        unsafe { *self.rightmost_leaf = *leaf };
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
            #[allow(clippy::manual_is_multiple_of)]
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

    /// Returns an iterator with changelog entries newer than the requested
    /// `changelog_index`.
    pub fn changelog_entries(
        &self,
        changelog_index: usize,
    ) -> Result<Skip<CyclicBoundedVecIterator<'_, ChangelogEntry<HEIGHT>>>, ConcurrentMerkleTreeError>
    {
        // `CyclicBoundedVec::iter_from` returns an iterator which includes also
        // the element indicated by the provided index.
        //
        // However, we want to iterate only on changelog events **newer** than
        // the provided one.
        //
        // Calling `iter_from(changelog_index + 1)` wouldn't work. If
        // `changelog_index` points to the newest changelog entry,
        // `changelog_index + 1` would point to the **oldest** changelog entry.
        // That would result in iterating over the whole changelog - from the
        // oldest to the newest element.
        Ok(self.changelog.iter_from(changelog_index)?.skip(1))
    }

    /// Updates the given Merkle proof.
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
    pub fn update_proof_from_changelog(
        &self,
        changelog_index: usize,
        leaf_index: usize,
        proof: &mut BoundedVec<[u8; 32]>,
    ) -> Result<(), ConcurrentMerkleTreeError> {
        // Iterate over changelog entries starting from the requested
        // `changelog_index`.
        //
        // Since we are interested only in subsequent, new changelog entries,
        // skip the first result.
        for changelog_entry in self.changelog_entries(changelog_index)? {
            changelog_entry.update_proof(leaf_index, proof)?;
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
        let expected_root = self.root();
        let computed_root = compute_root::<H>(leaf, leaf_index, proof)?;
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
    ) -> Result<(usize, usize), ConcurrentMerkleTreeError> {
        let mut changelog_entry = ChangelogEntry::default_with_index(leaf_index);
        let mut current_node = *new_leaf;
        for (level, sibling) in proof.iter().enumerate() {
            changelog_entry.path[level] = Some(current_node);
            current_node = compute_parent_node::<H>(&current_node, sibling, leaf_index, level)?;
        }

        self.inc_sequence_number()?;

        self.roots.push(current_node);

        // Check if the leaf is the last leaf in the tree.
        if self.next_index() < (1 << self.height) {
            changelog_entry.update_proof(self.next_index(), &mut self.filled_subtrees)?;
            // Check if we updated the rightmost leaf.
            if leaf_index >= self.current_index() {
                self.set_rightmost_leaf(new_leaf);
            }
        }
        self.changelog.push(changelog_entry);

        if self.canopy_depth > 0 {
            self.update_canopy(self.changelog.last_index(), 1);
        }

        Ok((self.changelog.last_index(), self.sequence_number()))
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
    ) -> Result<(usize, usize), ConcurrentMerkleTreeError> {
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
        if changelog_index != self.changelog_index() {
            self.update_proof_from_changelog(changelog_index, leaf_index, proof)?;
        }
        self.validate_proof(old_leaf, leaf_index, proof)?;
        self.update_leaf_in_tree(new_leaf, leaf_index, proof)
    }

    /// Appends a new leaf to the tree.
    pub fn append(&mut self, leaf: &[u8; 32]) -> Result<(usize, usize), ConcurrentMerkleTreeError> {
        self.append_batch(&[leaf])
    }

    /// Appends a new leaf to the tree. Saves Merkle proof to the provided
    /// `proof` reference.
    pub fn append_with_proof(
        &mut self,
        leaf: &[u8; 32],
        proof: &mut BoundedVec<[u8; 32]>,
    ) -> Result<(usize, usize), ConcurrentMerkleTreeError> {
        self.append_batch_with_proofs(&[leaf], &mut [proof])
    }

    /// Appends a batch of new leaves to the tree.
    pub fn append_batch(
        &mut self,
        leaves: &[&[u8; 32]],
    ) -> Result<(usize, usize), ConcurrentMerkleTreeError> {
        self.append_batch_common::<false>(leaves, None)
    }

    /// Appends a batch of new leaves to the tree. Saves Merkle proofs to the
    /// provided `proofs` slice.
    pub fn append_batch_with_proofs(
        &mut self,
        leaves: &[&[u8; 32]],
        proofs: &mut [&mut BoundedVec<[u8; 32]>],
    ) -> Result<(usize, usize), ConcurrentMerkleTreeError> {
        self.append_batch_common::<true>(leaves, Some(proofs))
    }

    /// Appends a batch of new leaves to the tree.
    ///
    /// This method contains the common logic and is not intended for external
    /// use. Callers should choose between [`append_batch`](ConcurrentMerkleTree::append_batch)
    /// and [`append_batch_with_proofs`](ConcurrentMerkleTree::append_batch_with_proofs).
    fn append_batch_common<
        // The only purpose of this const generic is to force compiler to
        // produce separate functions, with and without proof.
        //
        // Unfortunately, using `Option` is not enough:
        //
        // https://godbolt.org/z/fEMMfMdPc
        // https://godbolt.org/z/T3dxnjMzz
        //
        // Using the const generic helps and ends up generating two separate
        // functions:
        //
        // https://godbolt.org/z/zGnM7Ycn1
        const WITH_PROOFS: bool,
    >(
        &mut self,
        leaves: &[&[u8; 32]],
        // Slice for saving Merkle proofs.
        //
        // Currently it's used only for indexed Merkle trees.
        mut proofs: Option<&mut [&mut BoundedVec<[u8; 32]>]>,
    ) -> Result<(usize, usize), ConcurrentMerkleTreeError> {
        if leaves.is_empty() {
            return Err(ConcurrentMerkleTreeError::EmptyLeaves);
        }
        if (self.next_index() + leaves.len() - 1) >= 1 << self.height {
            return Err(ConcurrentMerkleTreeError::TreeIsFull);
        }
        if leaves.len() > self.changelog.capacity() {
            return Err(ConcurrentMerkleTreeError::BatchGreaterThanChangelog(
                leaves.len(),
                self.changelog.capacity(),
            ));
        }

        let first_changelog_index = (self.changelog.last_index() + 1) % self.changelog.capacity();
        let first_sequence_number = self.sequence_number() + 1;

        for (leaf_i, leaf) in leaves.iter().enumerate() {
            let mut current_index = self.next_index();

            self.changelog
                .push(ChangelogEntry::<HEIGHT>::default_with_index(current_index));
            let changelog_index = self.changelog_index();

            let mut current_node = **leaf;

            self.changelog[changelog_index].path[0] = Some(**leaf);

            for i in 0..self.height {
                #[allow(clippy::manual_is_multiple_of)]
                let is_left = current_index % 2 == 0;

                if is_left {
                    // If the current node is on the left side:
                    //
                    //     U
                    //    / \
                    //  CUR  SIB
                    //  /     \
                    // N       N
                    //
                    // * The sibling (on the right) is a "zero node".
                    // * That "zero node" becomes a part of Merkle proof.
                    // * The upper (next current) node is `H(cur, Ø)`.
                    let empty_node = H::zero_bytes()[i];

                    if WITH_PROOFS {
                        // PANICS: `proofs` should be always `Some` at this point.
                        proofs.as_mut().unwrap()[leaf_i].push(empty_node)?;
                    }

                    self.filled_subtrees[i] = current_node;

                    // For all non-terminal leaves, stop computing parents as
                    // soon as we are on the left side.
                    // Computation of the parent nodes is going to happen in
                    // the next iterations.
                    if leaf_i < leaves.len() - 1 {
                        break;
                    }

                    current_node = H::hashv(&[&current_node, &empty_node])?;
                } else {
                    // If the current node is on the right side:
                    //
                    //     U
                    //    / \
                    //  SIB  CUR
                    //  /     \
                    // N       N
                    // * The sigling on the left is a "filled subtree".
                    // * That "filled subtree" becomes a part of Merkle proof.
                    // * The upper (next current) node is `H(sib, cur)`.

                    if WITH_PROOFS {
                        // PANICS: `proofs` should be always `Some` at this point.
                        proofs.as_mut().unwrap()[leaf_i].push(self.filled_subtrees[i])?;
                    }

                    current_node = H::hashv(&[&self.filled_subtrees[i], &current_node])?;
                }

                if i < self.height - 1 {
                    self.changelog[changelog_index].path[i + 1] = Some(current_node);
                }

                current_index /= 2;
            }

            if leaf_i == leaves.len() - 1 {
                self.roots.push(current_node);
            } else {
                // Photon returns only the sequence number and we use it in the
                // JS client and forester to derive the root index. Therefore,
                // we need to emit a "zero root" to not break that property.
                self.roots.push([0u8; 32]);
            }

            self.inc_next_index()?;
            self.inc_sequence_number()?;

            self.set_rightmost_leaf(leaf);
        }

        if self.canopy_depth > 0 {
            self.update_canopy(first_changelog_index, leaves.len());
        }

        Ok((first_changelog_index, first_sequence_number))
    }

    fn update_canopy(&mut self, first_changelog_index: usize, num_leaves: usize) {
        for i in 0..num_leaves {
            let changelog_index = (first_changelog_index + i) % self.changelog.capacity();
            for (i, path_node) in self.changelog[changelog_index]
                .path
                .iter()
                .rev()
                .take(self.canopy_depth)
                .enumerate()
            {
                if let Some(path_node) = path_node {
                    let level = self.height - i - 1;
                    let index = (1 << (self.height - level))
                        + (self.changelog[changelog_index].index >> level);
                    // `index - 2` maps to the canopy index.
                    self.canopy[(index - 2) as usize] = *path_node;
                }
            }
        }
    }
}

impl<H, const HEIGHT: usize> Drop for ConcurrentMerkleTree<H, HEIGHT>
where
    H: Hasher,
{
    fn drop(&mut self) {
        let layout = Layout::new::<usize>();
        unsafe { alloc::dealloc(self.next_index as *mut u8, layout) };

        let layout = Layout::new::<usize>();
        unsafe { alloc::dealloc(self.sequence_number as *mut u8, layout) };

        let layout = Layout::new::<[u8; 32]>();
        unsafe { alloc::dealloc(self.rightmost_leaf as *mut u8, layout) };
    }
}

impl<H, const HEIGHT: usize> PartialEq for ConcurrentMerkleTree<H, HEIGHT>
where
    H: Hasher,
{
    fn eq(&self, other: &Self) -> bool {
        self.height.eq(&other.height)
            && self.canopy_depth.eq(&other.canopy_depth)
            && self.next_index().eq(&other.next_index())
            && self.sequence_number().eq(&other.sequence_number())
            && self.rightmost_leaf().eq(&other.rightmost_leaf())
            && self
                .filled_subtrees
                .as_slice()
                .eq(other.filled_subtrees.as_slice())
            && self.changelog.iter().eq(other.changelog.iter())
            && self.roots.iter().eq(other.roots.iter())
            && self.canopy.as_slice().eq(other.canopy.as_slice())
    }
}
