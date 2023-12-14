use std::marker::PhantomData;

use light_hasher::{errors::HasherError, Hasher};

use crate::{
    changelog::ChangelogEntry,
    hash::{compute_parent_node, full_proof, is_valid_proof},
};

pub type MerkleProof<const MAX_HEIGHT: usize> = [[u8; 32]; MAX_HEIGHT];

#[repr(C)]
pub struct ConcurrentMerkleTree<H, const MAX_HEIGHT: usize, const MAX_ROOTS: usize>
where
    H: Hasher,
{
    /// History of Merkle proofs.
    pub changelog: [ChangelogEntry<MAX_HEIGHT>; MAX_ROOTS],
    /// History of roots.
    pub roots: [[u8; 32]; MAX_ROOTS],
    /// Index of the newest changelog and root.
    pub current_index: u64,
    /// The newest Merkle proof.
    pub rightmost_proof: MerkleProof<MAX_HEIGHT>,
    pub rightmost_index: u64,
    pub rightmost_leaf: [u8; 32],

    _hasher: PhantomData<H>,
}

impl<H, const MAX_HEIGHT: usize, const MAX_ROOTS: usize> Default
    for ConcurrentMerkleTree<H, MAX_HEIGHT, MAX_ROOTS>
where
    H: Hasher,
{
    fn default() -> Self {
        Self {
            changelog: [ChangelogEntry::default(); MAX_ROOTS],
            roots: [[0u8; 32]; MAX_ROOTS],
            current_index: 0,
            rightmost_proof: [[0u8; 32]; MAX_HEIGHT],
            rightmost_index: 0,
            rightmost_leaf: [0u8; 32],
            _hasher: PhantomData,
        }
    }
}

impl<H, const MAX_HEIGHT: usize, const MAX_ROOTS: usize>
    ConcurrentMerkleTree<H, MAX_HEIGHT, MAX_ROOTS>
where
    H: Hasher,
{
    pub fn init(&mut self) {
        // Initialize changelog.
        let root = H::zero_bytes()[MAX_HEIGHT];
        let mut changelog_path = [[0u8; 32]; MAX_HEIGHT];
        for (i, leaf) in changelog_path.iter_mut().enumerate() {
            *leaf = H::zero_bytes()[i];
        }
        let changelog_entry = ChangelogEntry::new(root, changelog_path, 0);
        self.changelog[0] = changelog_entry;

        // Initialize root.
        self.roots[0] = root;

        // Initialize rightmost proof and leaf.
        for (i, leaf) in self.rightmost_proof.iter_mut().enumerate() {
            *leaf = H::zero_bytes()[i];
        }
        // self.rightmost_index += 1;
    }

    pub fn root(&self) -> [u8; 32] {
        self.roots[self.current_index as usize]
    }

    pub fn update_path_to_leaf(
        &mut self,
        mut leaf: [u8; 32],
        i: usize,
        proof: &[[u8; 32]],
    ) -> Result<(), HasherError> {
        let mut changelog_path = [[0u8; 32]; MAX_HEIGHT];

        for (j, sibling) in proof.iter().enumerate() {
            // PushBack(changeLog.path, node)
            changelog_path[j] = leaf;
            leaf = compute_parent_node::<H>(&leaf, sibling, i, j)?;
        }

        // PushFront(tree.changeLogs, changeLog)
        // PushFront(tree.rootBuffer, node)
        let changelog = ChangelogEntry::new(leaf, changelog_path, i);
        self.current_index += 1;
        self.changelog[self.current_index as usize] = changelog;
        self.roots[self.current_index as usize] = leaf;

        // if i >= self.rightmost_index as usize {
        //     println!("updating proof");
        //     self.rightmost_proof.copy_from_slice(proof);
        //     // self.rightmost_index += 1;
        //     // self.rightmost_leaf = leaf;
        //     println!("rightmost proof after update: {:?}", self.rightmost_proof);
        // }

        Ok(())
    }

    fn replace_leaf_inner(
        &mut self,
        old_leaf: [u8; 32],
        new_leaf: [u8; 32],
        leaf_index: usize,
        root_index: usize,
        proof: &[[u8; 32]; MAX_HEIGHT],
    ) -> Result<(), HasherError> {
        let mut updated_leaf = old_leaf;
        let mut updated_proof = proof.clone();
        for k in root_index..(self.current_index as usize) + 1 {
            let changelog_entry = self.changelog[k];
            if leaf_index != changelog_entry.index as usize {
                // This bit math is used to identify which node in the proof
                // we need to swap for a corresponding node in a saved change log
                let padding = 64 - MAX_HEIGHT;
                let common_path_len = ((leaf_index ^ changelog_entry.index as usize) << padding)
                    .leading_zeros() as usize;
                let critbit_index = (MAX_HEIGHT - 1) - common_path_len;

                updated_proof[critbit_index] = changelog_entry.path[critbit_index];
            } else {
                updated_leaf = changelog_entry.path[0];
            }
        }
        if is_valid_proof::<H, MAX_HEIGHT>(
            self.roots[self.current_index as usize],
            old_leaf,
            leaf_index,
            proof,
        )? && updated_leaf == old_leaf
        {
            self.update_path_to_leaf(new_leaf, leaf_index, &updated_proof)?
        }

        Ok(())
    }

    /// Replaces the `old_leaf` under the `leaf_index` with a `new_leaf`, using
    /// the given `root` and `proof`.
    pub fn replace_leaf(
        &mut self,
        root: [u8; 32],
        old_leaf: &[u8],
        new_leaf: &[u8],
        leaf_index: usize,
        proof: &[[u8; 32]],
    ) -> Result<(), HasherError> {
        let old_leaf = H::hash(old_leaf)?;
        let new_leaf = H::hash(new_leaf)?;

        let proof = full_proof::<H, MAX_HEIGHT>(proof);

        for root_index in 0..(self.current_index as usize) + 1 {
            if root == self.roots[root_index] {
                self.replace_leaf_inner(old_leaf, new_leaf, leaf_index, root_index, &proof)?;
            }
        }

        Ok(())
    }

    /// Appends a new leaf to the tree.
    pub fn append(&mut self, leaf: &[u8]) -> Result<(), HasherError> {
        let leaf = H::hash(leaf)?;

        let mut changelog_path = [[0u8; 32]; MAX_HEIGHT];
        let mut intersection_node = self.rightmost_leaf;
        let intersection_index = self.rightmost_index.trailing_zeros() as usize;

        if self.rightmost_index == 0 {
            // NOTE(vadorovsky): This is not mentioned in the whitepaper, but
            // appending to an empty Merkle tree is a special case, where
            // `computer_parent_node` can't be called, because the usual
            // `self.rightmost_index - 1` used as a sibling index would be a
            //  negative value.
            //
            // [spl-concurrent-merkle-tree]()
            // seems to handle this case by:
            //
            // * Valitating a proof.
            // * Performing procedures which usually are done by `replace_leaf`
            //   algorithm.
            //
            // Here, we just call compute the new root and call `replace_leaf`
            // directly.
            // leaf = H::hashv(&[&leaf, &H::zero_bytes()[1]])?;
            let proof = self.rightmost_proof.clone();
            self.replace_leaf_inner(H::zero_bytes()[0], leaf, 0, 0, &proof)?;
        } else {
            let mut current_leaf = leaf;

            for (i, item) in changelog_path.iter_mut().enumerate() {
                *item = current_leaf;

                if i < intersection_index {
                    let empty_node = H::zero_bytes()[i];
                    current_leaf = H::hashv(&[&current_leaf, &empty_node])?;
                    intersection_node = compute_parent_node::<H>(
                        &intersection_node,
                        &self.rightmost_proof[i],
                        self.rightmost_index as usize - 1,
                        i,
                    )?;
                    self.rightmost_proof[i] = empty_node;
                } else if i == intersection_index {
                    current_leaf = H::hashv(&[&intersection_node, &current_leaf])?;
                    self.rightmost_proof[i] = intersection_node;
                } else {
                    current_leaf = compute_parent_node::<H>(
                        &current_leaf,
                        &self.rightmost_proof[i],
                        self.rightmost_index as usize - 1,
                        i,
                    )?;
                }
            }

            self.current_index += 1;
            self.changelog[self.current_index as usize] =
                ChangelogEntry::new(leaf, changelog_path, self.rightmost_index as usize);
            self.roots[self.current_index as usize] = current_leaf;
        }

        self.rightmost_index += 1;
        self.rightmost_leaf = leaf;

        Ok(())
    }
}
