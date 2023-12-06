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
        println!("setting leaf {leaf:?} as root");
        self.roots[self.current_index as usize] = leaf;

        Ok(())
    }

    fn replace_leaf_inner(
        &mut self,
        old_leaf: [u8; 32],
        new_leaf: [u8; 32],
        i: usize,
        j: usize,
        proof: &[[u8; 32]; MAX_HEIGHT],
    ) -> Result<(), HasherError> {
        let mut updated_leaf = old_leaf;
        let mut updated_proof = proof.clone();
        for k in j..(self.current_index as usize) + 1 {
            println!("iteration k: {k}");
            let changelog_entry = self.changelog[k];
            if i != changelog_entry.index as usize {
                println!("swapping critbit");
                // This bit math is used to identify which node in the proof
                // we need to swap for a corresponding node in a saved change log
                let padding = 32 - MAX_HEIGHT;
                let common_path_len =
                    ((i ^ changelog_entry.index as usize) << padding).leading_zeros() as usize;
                let critbit_index = (MAX_HEIGHT - 1) - common_path_len;

                updated_proof[critbit_index] = changelog_entry.path[critbit_index];
            } else {
                println!("not swapping critbit");
                updated_leaf = changelog_entry.path[0];
            }
        }
        println!("about to validate proof");
        println!("old_leaf: {old_leaf:?}, updated_leaf: {updated_leaf:?}");
        if is_valid_proof::<H, MAX_HEIGHT>(
            self.roots[self.current_index as usize],
            old_leaf,
            i,
            proof,
        )? && updated_leaf == old_leaf
        {
            println!("proof valid");
            self.update_path_to_leaf(new_leaf, i, &updated_proof)?
        }

        Ok(())
    }

    pub fn replace_leaf(
        &mut self,
        root: [u8; 32],
        old_leaf: [u8; 32],
        new_leaf: [u8; 32],
        i: usize,
        proof: &[[u8; 32]],
    ) -> Result<(), HasherError> {
        let proof = full_proof::<H, MAX_HEIGHT>(proof);
        // let mut updated_proof = proof.clone();

        for j in 0..(self.current_index as usize) + 1 {
            println!(
                "iteration j: {j}, root: {root:?}, roots[j]: {:?}",
                self.roots[j]
            );
            if root == self.roots[j] {
                self.replace_leaf_inner(old_leaf, new_leaf, i, j, &proof)?;
            }
        }

        Ok(())
    }

    /// Appends a new node to the tree.
    pub fn append(&mut self, mut leaf: [u8; 32]) -> Result<(), HasherError> {
        let mut changelog_path = [[0u8; 32]; MAX_HEIGHT];
        let mut intersection_node = self.rightmost_leaf;
        let intersection_index = self.rightmost_index.trailing_zeros() as usize;

        if self.rightmost_index == 0 {
            println!("FIRST INSERTION");
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
            leaf = H::hashv(&[&leaf, &H::zero_bytes()[1]])?;
            let proof = self.rightmost_proof.clone();
            self.replace_leaf_inner(H::zero_bytes()[0], leaf, 0, 0, &proof)?;
        } else {
            for (i, item) in changelog_path.iter_mut().enumerate() {
                *item = leaf;

                println!("i: {i}, intersection_index: {intersection_index}");
                if i < intersection_index {
                    let empty_node = H::zero_bytes()[i];
                    leaf = H::hashv(&[&leaf, &empty_node])?;
                    intersection_node = compute_parent_node::<H>(
                        &intersection_node,
                        &self.rightmost_proof[i],
                        self.rightmost_index as usize - 1,
                        i,
                    )?;
                    self.rightmost_proof[i] = empty_node;
                } else if i == intersection_index {
                    leaf = H::hashv(&[&intersection_node, &leaf])?;
                } else {
                    leaf = compute_parent_node::<H>(
                        &leaf,
                        &self.rightmost_proof[i],
                        self.rightmost_index as usize - 1,
                        i,
                    )?;
                }
            }

            self.current_index += 1;
            self.changelog[self.current_index as usize] =
                ChangelogEntry::new(leaf, changelog_path, self.rightmost_index as usize);
            self.roots[self.current_index as usize] = leaf;
        }

        self.rightmost_index += 1;
        self.rightmost_leaf = leaf;

        Ok(())
    }
}
