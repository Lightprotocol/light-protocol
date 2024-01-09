use std::{cmp::Ordering, marker::PhantomData};

use light_hasher::{errors::HasherError, Hasher};

pub mod changelog;
pub mod hash;

use crate::{
    changelog::ChangelogEntry,
    hash::{compute_parent_node, validate_proof},
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
pub struct ConcurrentMerkleTree<
    H,
    const HEIGHT: usize,
    const MAX_CHANGELOG: usize,
    const MAX_ROOTS: usize,
> where
    H: Hasher,
{
    /// History of Merkle proofs.
    pub changelog: [ChangelogEntry<HEIGHT>; MAX_CHANGELOG],
    /// Index of the newest changelog.
    pub current_changelog_index: u64,
    /// History of roots.
    pub roots: [[u8; 32]; MAX_ROOTS],
    /// Index of the newest root.
    pub current_root_index: u64,
    /// The newest Merkle proof.
    pub rightmost_proof: [[u8; 32]; HEIGHT],
    /// Index of the newest non-empty leaf.
    pub rightmost_index: u64,
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
            current_root_index: 0,
            rightmost_proof: [[0u8; 32]; HEIGHT],
            rightmost_index: 0,
            rightmost_leaf: [0u8; 32],
            _hasher: PhantomData,
        }
    }
}

impl<H, const HEIGHT: usize, const MAX_CHANGELOG: usize, const MAX_ROOTS: usize>
    ConcurrentMerkleTree<H, HEIGHT, MAX_CHANGELOG, MAX_ROOTS>
where
    H: Hasher,
{
    /// Initializes the Merkle tree.
    pub fn init(&mut self) -> Result<(), HasherError> {
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
        *self.roots.get_mut(0).ok_or(HasherError::RootsZero)? = root;

        // Initialize rightmost proof.
        for (i, node) in self.rightmost_proof.iter_mut().enumerate() {
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
    pub fn root(&self) -> Result<[u8; 32], HasherError> {
        self.roots
            .get(self.current_root_index as usize)
            .ok_or(HasherError::RootHigherThanMax)
            .map(|&value| value)
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
    fn update_proof_or_leaf(
        &self,
        changelog_index: usize,
        leaf_index: usize,
        proof: &[[u8; 32]; HEIGHT],
    ) -> Option<[[u8; 32]; HEIGHT]> {
        let mut updated_proof = proof.to_owned();

        let mut i = changelog_index + 1;

        while i != self.current_changelog_index as usize + 1 {
            let changelog_entry = self.changelog[i];

            updated_proof = match changelog_entry.update_proof(leaf_index, &updated_proof) {
                Some(proof) => proof,
                None => return None,
            };

            i = (i + 1) % MAX_ROOTS;
        }

        Some(updated_proof)
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
    ) -> Result<(), HasherError> {
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
            .ok_or(HasherError::RootsZero)? = node;

        // Update the rightmost proof. It has to be done only if tree is not full.
        if self.rightmost_index < (1 << HEIGHT) {
            if self.rightmost_index > 0 && leaf_index < self.rightmost_index as usize - 1 {
                // Update the rightmost proof with the current changelog entry when:
                //
                // * `rightmost_index` is greater than 0 (tree is non-empty).
                // * The updated leaf is non-rightmost.
                if let Some(proof) = changelog_entry
                    .update_proof(self.rightmost_index as usize - 1, &self.rightmost_proof)
                {
                    self.rightmost_proof = proof;
                }
            } else {
                // Save the provided proof and leaf as the new rightmost under
                // any of the following conditions:
                //
                // * Tree is empty (and this is the first `append`).
                // * The rightmost leaf is updated.
                self.rightmost_proof.copy_from_slice(proof);
                self.rightmost_leaf = *new_leaf;
            }
        }

        Ok(())
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
    ) -> Result<(), HasherError> {
        let updated_proof = if self.rightmost_index > 0 && MAX_CHANGELOG > 0 {
            match self.update_proof_or_leaf(changelog_index, leaf_index, proof) {
                Some(proof) => proof,
                // This case means that the leaf we are trying to update was
                // already updated. Therefore, updating the proof is impossible.
                // We need to return an error and request the caller
                // to retry the update with a new proof.
                None => {
                    return Err(HasherError::CannotUpdateLeaf);
                }
            }
        } else {
            if leaf_index != self.rightmost_index as usize {
                return Err(HasherError::AppendOnly);
            }
            proof.to_owned()
        };

        validate_proof::<H, HEIGHT>(
            &self.roots[self.current_root_index as usize],
            old_leaf,
            leaf_index,
            proof,
        )?;
        self.update_leaf_in_tree(new_leaf, leaf_index, &updated_proof)
    }

    /// Appends a new leaf to the tree.
    pub fn append(&mut self, leaf: &[u8; 32]) -> Result<(), HasherError> {
        if self.rightmost_index >= 1 << HEIGHT {
            return Err(HasherError::TreeFull);
        }

        if self.rightmost_index == 0 {
            // NOTE(vadorovsky): This is not mentioned in the whitepaper, but
            // appending to an empty Merkle tree is a special case, where
            // `computer_parent_node` can't be called, because the usual
            // `self.rightmost_index - 1` used as a sibling index would be a
            //  negative value.
            //
            // [spl-concurrent-merkle-tree](https://github.com/solana-labs/solana-program-library/blob/da94833aa16d756aed49ee1a7aa295295b41d19a/libraries/concurrent-merkle-tree/src/concurrent_merkle_tree.rs#L263-L265)
            // handles this case by:
            //
            // * Valitating a proof.
            // * Performing procedures which usually are done by `replace_leaf`
            //   algorithm.
            //
            // Here, we just call `update` directly, because we wrote it in a
            // way which allows an "update" of the 1st leaf in the empty tree.
            let proof = self.rightmost_proof;
            self.update(0, &H::zero_bytes()[0], leaf, 0, &proof)?;
        } else {
            let mut current_node = *leaf;
            let mut intersection_node = self.rightmost_leaf;
            let intersection_index = self.rightmost_index.trailing_zeros() as usize;
            let mut changelog_path = [[0u8; 32]; HEIGHT];

            for (i, item) in changelog_path.iter_mut().enumerate() {
                *item = current_node;

                match i.cmp(&intersection_index) {
                    Ordering::Less => {
                        let empty_node = H::zero_bytes()[i];
                        current_node = H::hashv(&[&current_node, &empty_node])?;
                        intersection_node = compute_parent_node::<H>(
                            &intersection_node,
                            &self.rightmost_proof[i],
                            self.rightmost_index as usize - 1,
                            i,
                        )?;
                        self.rightmost_proof[i] = empty_node;
                    }
                    Ordering::Equal => {
                        current_node = H::hashv(&[&intersection_node, &current_node])?;
                        self.rightmost_proof[intersection_index] = intersection_node;
                    }
                    Ordering::Greater => {
                        current_node = compute_parent_node::<H>(
                            &current_node,
                            &self.rightmost_proof[i],
                            self.rightmost_index as usize - 1,
                            i,
                        )?;
                    }
                }
            }

            self.inc_current_changelog_index();
            if let Some(changelog_element) = self
                .changelog
                .get_mut(self.current_changelog_index as usize)
            {
                *changelog_element =
                    ChangelogEntry::new(current_node, changelog_path, self.rightmost_index as usize)
            }
            self.inc_current_root_index();
            *self
                .roots
                .get_mut(self.current_root_index as usize)
                .ok_or(HasherError::RootsZero)? = current_node;
        }

        self.rightmost_index += 1;
        self.rightmost_leaf = *leaf;

        Ok(())
    }
}
