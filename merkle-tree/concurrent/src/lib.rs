use std::marker::PhantomData;

use bytemuck::{Pod, Zeroable};
use hash::compute_root;
pub use light_hasher;
use light_hasher::Hasher;

pub mod changelog;
pub mod errors;
pub mod hash;

use crate::{
    changelog::{ChangelogEntry, MerklePaths},
    errors::ConcurrentMerkleTreeError,
    hash::compute_parent_node,
};

/// [Concurrent Merkle tree](https://drive.google.com/file/d/1BOpa5OFmara50fTvL0VIVYjtg-qzHCVc/view)
/// which allows for multiple requests of updating leaves, without making any
/// of the requests invalid, as long as they are not:
///
/// * Modyfing the same leaf.
/// * Exceeding the capacity of the `changelog` (`MAX_CHANGELOG`).
///
/// When any of the above happens, some of the concurrent requests are going to
/// be invalid, forcing the clients to re-generate the Merkle proof. But that's
/// still better than having such a failure after any update happening in the
/// middle of requesting the update.
///
/// Due to ability to make a decent number of concurrent update requests to be
/// valid, no lock is necessary.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ConcurrentMerkleTree<
    H,
    const HEIGHT: usize,
    const MAX_CHANGELOG: usize,
    const MAX_ROOTS: usize,
> where
    H: Hasher,
{
    /// Index of the newest non-empty leaf.
    pub next_index: u64,
    /// History of roots.
    pub roots: [[u8; 32]; MAX_ROOTS],
    /// Number of successful operations on the tree.
    pub sequence_number: u64,
    /// History of Merkle proofs.
    pub changelog: [ChangelogEntry<HEIGHT>; MAX_CHANGELOG],
    /// Index of the newest changelog.
    pub current_changelog_index: u64,
    /// Index of the newest root.
    pub current_root_index: u64,
    /// The newest Merkle proof.
    pub filled_subtrees: [[u8; 32]; HEIGHT],
    /// The newest non-empty leaf.
    pub rightmost_leaf: [u8; 32],

    _hasher: PhantomData<H>,
}

impl<H, const HEIGHT: usize, const MAX_CHANGELOG: usize, const MAX_ROOTS: usize> Default
    for ConcurrentMerkleTree<H, HEIGHT, MAX_CHANGELOG, MAX_ROOTS>
where
    H: Hasher,
{
    fn default() -> Self {
        Self {
            changelog: [ChangelogEntry::default(); MAX_CHANGELOG],
            current_changelog_index: 0,
            roots: [[0u8; 32]; MAX_ROOTS],
            sequence_number: 0,
            current_root_index: 0,
            filled_subtrees: [[0u8; 32]; HEIGHT],
            next_index: 0,
            rightmost_leaf: [0u8; 32],
            _hasher: PhantomData,
        }
    }
}

/// Mark `ConcurrentMerkleTree` as `Zeroable`, providing Anchor a guarantee
/// that it can be always initialized with zeros.
///
/// # Safety
///
/// [`bytemuck`](bytemuck) is not able to ensure that our custom types (`Hasher`
/// and `ConcurrentMerkleTree`) can be a subject of initializing with zeros. It
/// also doesn't support structs with const generics (it would need to ensure
/// alignment).
///
/// Therefore, it's our responsibility to guarantee that `ConcurrentMerkleTree`
/// doesn't contain any fields which are not zeroable.
unsafe impl<H, const HEIGHT: usize, const MAX_CHANGELOG: usize, const MAX_ROOTS: usize> Zeroable
    for ConcurrentMerkleTree<H, HEIGHT, MAX_CHANGELOG, MAX_ROOTS>
where
    H: Hasher,
{
}

/// Mark `ConcurrentMerkleTree` as `Pod` (Plain Old Data), providing Anchor a
/// guarantee that it can be used in a zero-copy account.
///
/// # Safety
///
/// [`bytemuck`](bytemuck) is not able to ensure that our custom types (`Hasher`
/// and `ConcurrentMerkleTree`) can be a subject of byte serialization. It also
/// doesn't support structs with const generics (it would need to ensure
/// alignment).
///
/// Therefore, it's our responsibility to guarantee that:
///
/// * `Hasher` and `ConcurrentMerkleTree` with given const generics are aligned.
/// * They don't contain any fields which are not implementing `Copy` or are
///   not an easy subject for byte serialization.
unsafe impl<H, const HEIGHT: usize, const MAX_CHANGELOG: usize, const MAX_ROOTS: usize> Pod
    for ConcurrentMerkleTree<H, HEIGHT, MAX_CHANGELOG, MAX_ROOTS>
where
    H: Hasher + Copy + 'static,
{
}

impl<H, const HEIGHT: usize, const MAX_CHANGELOG: usize, const MAX_ROOTS: usize>
    ConcurrentMerkleTree<H, HEIGHT, MAX_CHANGELOG, MAX_ROOTS>
where
    H: Hasher,
{
    /// Initializes the Merkle tree.
    pub fn init(&mut self) -> Result<(), ConcurrentMerkleTreeError> {
        // Initialize changelog.
        let root = H::zero_bytes()[HEIGHT];
        let mut changelog_path = [[0u8; 32]; HEIGHT];
        for (i, node) in changelog_path.iter_mut().enumerate() {
            *node = H::zero_bytes()[i];
        }
        let changelog_entry = ChangelogEntry::new(root, changelog_path, 0);
        if let Some(changelog_element) = self.changelog.get_mut(0) {
            *changelog_element = changelog_entry;
        }

        // Initialize root.
        *self
            .roots
            .get_mut(0)
            .ok_or(ConcurrentMerkleTreeError::RootsZero)? = root;

        // Initialize rightmost proof.
        for (i, node) in self.filled_subtrees.iter_mut().enumerate() {
            *node = H::zero_bytes()[i];
        }

        Ok(())
    }

    /// Increments the changelog counter. If it reaches the limit, it starts
    /// from the beginning.
    fn inc_current_changelog_index(&mut self) {
        // NOTE(vadorovsky): Apparenty, Rust doesn't have `checked_remainder`
        // or anything like that.
        self.current_changelog_index = if MAX_CHANGELOG > 0 {
            (self.current_changelog_index + 1) % MAX_CHANGELOG as u64
        } else {
            0
        };
    }

    /// Increments the root counter. If it reaches the limit, it starts from
    /// the beginning.
    fn inc_current_root_index(&mut self) {
        self.current_root_index = (self.current_root_index + 1) % MAX_ROOTS as u64;
    }

    /// Returns the index of the current changelog entry.
    pub fn changelog_index(&self) -> usize {
        self.current_changelog_index as usize
    }

    /// Returns the index of the current root in the tree's root buffer.
    pub fn root_index(&self) -> usize {
        self.current_root_index as usize
    }

    /// Returns the current root.
    pub fn root(&self) -> Result<[u8; 32], ConcurrentMerkleTreeError> {
        self.roots
            .get(self.current_root_index as usize)
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
        self.next_index as usize
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
    fn update_proof(
        &self,
        changelog_index: usize,
        leaf_index: usize,
        proof: &mut [[u8; 32]; HEIGHT],
    ) -> Result<(), ConcurrentMerkleTreeError> {
        let mut i = changelog_index + 1;

        while i != self.current_changelog_index as usize + 1 {
            let changelog_entry = self.changelog[i];

            changelog_entry.update_proof(leaf_index, proof)?;

            i = (i + 1) % MAX_ROOTS;
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
        proof: &[[u8; 32]; HEIGHT],
    ) -> Result<(), ConcurrentMerkleTreeError> {
        let expected_root = self.root()?;
        let computed_root = compute_root::<H, HEIGHT>(leaf, leaf_index, proof)?;
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
        proof: &[[u8; 32]; HEIGHT],
    ) -> Result<ChangelogEntry<HEIGHT>, ConcurrentMerkleTreeError> {
        let mut node = *new_leaf;
        let mut changelog_path = [[0u8; 32]; HEIGHT];

        for (j, sibling) in proof.iter().enumerate() {
            changelog_path[j] = node;
            node = compute_parent_node::<H>(&node, sibling, leaf_index, j)?;
        }

        let changelog_entry = ChangelogEntry::new(node, changelog_path, leaf_index);
        self.inc_current_changelog_index();
        if let Some(changelog_element) = self
            .changelog
            .get_mut(self.current_changelog_index as usize)
        {
            *changelog_element = changelog_entry
        }

        self.inc_current_root_index();
        *self
            .roots
            .get_mut(self.current_root_index as usize)
            .ok_or(ConcurrentMerkleTreeError::RootsZero)? = node;

        changelog_entry.update_subtrees(self.next_index as usize - 1, &mut self.filled_subtrees);

        // Check if we updated the rightmost leaf.
        if self.next_index() < (1 << HEIGHT) && leaf_index >= self.current_index() {
            self.rightmost_leaf = *new_leaf;
        }

        Ok(changelog_entry)
    }

    /// Replaces the `old_leaf` under the `leaf_index` with a `new_leaf`, using
    /// the given `proof` and `changelog_index` (pointing to the changelog entry
    /// which was the newest at the time of preparing the proof).
    pub fn update(
        &mut self,
        changelog_index: usize,
        old_leaf: &[u8; 32],
        new_leaf: &[u8; 32],
        leaf_index: usize,
        proof: &[[u8; 32]; HEIGHT],
    ) -> Result<ChangelogEntry<HEIGHT>, ConcurrentMerkleTreeError> {
        if leaf_index >= self.next_index() {
            return Err(ConcurrentMerkleTreeError::CannotUpdateEmpty);
        }

        let mut proof = proof.to_owned();

        if MAX_CHANGELOG > 0 {
            self.update_proof(changelog_index, leaf_index, &mut proof)?;
        }
        self.validate_proof(old_leaf, leaf_index, &proof)?;
        self.update_leaf_in_tree(new_leaf, leaf_index, &proof)
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
        if (self.next_index as usize + leaves.len() - 1) >= 1 << HEIGHT {
            return Err(ConcurrentMerkleTreeError::TreeFull);
        }

        let first_leaf_index = self.next_index;
        // Buffer of Merkle paths.
        let mut merkle_paths = MerklePaths::<H, HEIGHT>::new(leaves.len());

        for (leaf_i, leaf) in leaves.iter().enumerate() {
            let mut current_index = self.next_index;
            let mut current_node = leaf.to_owned().to_owned();

            if leaf_i > 0 {
                merkle_paths.inc_current_leaf();
            }

            // Limit until which we fill up the current Merkle path.
            let fillup_index = if leaf_i < (leaves.len() - 1) {
                self.next_index.trailing_ones() as usize + 1
            } else {
                HEIGHT
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

                if i < HEIGHT - 1 {
                    merkle_paths.set(i + 1, current_node);
                }

                current_index /= 2;
            }

            merkle_paths.set_root(current_node);

            self.inc_current_root_index();
            *self
                .roots
                .get_mut(self.current_root_index as usize)
                .ok_or(ConcurrentMerkleTreeError::RootsZero)? = current_node;

            self.sequence_number = self.sequence_number.saturating_add(1);
            self.next_index = self.next_index.saturating_add(1);
            self.rightmost_leaf = leaf.to_owned().to_owned();
        }

        let changelog_entries = merkle_paths.to_changelog_entries(first_leaf_index as usize)?;

        // Save changelog entries.
        for changelog_entry in changelog_entries.iter() {
            self.inc_current_changelog_index();
            if let Some(changelog_element) = self
                .changelog
                .get_mut(self.current_changelog_index as usize)
            {
                *changelog_element = *changelog_entry;
            }
        }

        Ok(changelog_entries)
    }
}
