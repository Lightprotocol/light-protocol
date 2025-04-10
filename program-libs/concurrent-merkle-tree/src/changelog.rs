use std::ops::{Deref, DerefMut};

use light_bounded_vec::BoundedVec;

use crate::errors::ConcurrentMerkleTreeError;

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct ChangelogPath<const HEIGHT: usize>(pub [Option<[u8; 32]>; HEIGHT]);

impl<const HEIGHT: usize> ChangelogPath<HEIGHT> {
    pub fn from_fn<F>(cb: F) -> Self
    where
        F: FnMut(usize) -> Option<[u8; 32]>,
    {
        Self(std::array::from_fn(cb))
    }

    /// Checks whether the path is equal to the provided [`BoundedVec`].
    ///
    /// [`ChangelogPath`] might contain `None` nodes at the end, which
    /// mean that it does not define them, but the following changelog
    /// paths are expected to overwrite them.
    ///
    /// Therefore, the comparison ends on the first encountered first
    /// `None`. If all `Some` nodes are equal to the corresponding ones
    /// in the provided vector, the result is `true`.
    pub fn eq_to(&self, other: BoundedVec<[u8; 32]>) -> bool {
        if other.len() != HEIGHT {
            return false;
        }

        for i in 0..HEIGHT {
            let changelog_node = self.0[i];
            let path_node = other[i];
            match changelog_node {
                Some(changelog_node) => {
                    if changelog_node != path_node {
                        return false;
                    }
                }
                None => break,
            }
        }

        true
    }

    pub fn eq_to_vec(&self, other: Vec<[u8; 32]>) -> bool {
        if other.len() != HEIGHT {
            return false;
        }

        for (i, path_node) in other.iter().enumerate() {
            let changelog_node = self.0[i];
            match changelog_node {
                Some(changelog_node) => {
                    if changelog_node != *path_node {
                        return false;
                    }
                }
                None => break,
            }
        }

        true
    }
}

impl<const HEIGHT: usize> Default for ChangelogPath<HEIGHT> {
    fn default() -> Self {
        Self([None; HEIGHT])
    }
}

impl<const HEIGHT: usize> Deref for ChangelogPath<HEIGHT> {
    type Target = [Option<[u8; 32]>; HEIGHT];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const HEIGHT: usize> DerefMut for ChangelogPath<HEIGHT> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct ChangelogEntry<const HEIGHT: usize> {
    // Path of the changelog.
    pub path: ChangelogPath<HEIGHT>,
    // Index of the affected leaf.
    pub index: u64,
}

pub type ChangelogEntry22 = ChangelogEntry<22>;
pub type ChangelogEntry26 = ChangelogEntry<26>;
pub type ChangelogEntry32 = ChangelogEntry<32>;
pub type ChangelogEntry40 = ChangelogEntry<40>;

impl<const HEIGHT: usize> ChangelogEntry<HEIGHT> {
    pub fn new(path: ChangelogPath<HEIGHT>, index: usize) -> Self {
        let index = index as u64;
        Self { path, index }
    }

    pub fn default_with_index(index: usize) -> Self {
        Self {
            path: ChangelogPath::default(),
            index: index as u64,
        }
    }

    pub fn index(&self) -> usize {
        self.index as usize
    }

    /// Returns an intersection index in the changelog entry which affects the
    /// provided path.
    ///
    /// Determining it can be done by taking a XOR of the leaf index (which was
    /// directly updated in the changelog entry) and the leaf index we are
    /// trying to update.
    ///
    /// The number of bytes in the binary representations of the indexes is
    /// determined by the height of the tree. For example, for the tree with
    /// height 4, update attempt of leaf under index 2 and changelog affecting
    /// index 4, critbit would be:
    ///
    /// 2 ^ 4 = 0b_0010 ^ 0b_0100 = 0b_0110 = 6
    fn intersection_index(&self, leaf_index: usize) -> usize {
        let padding = 64 - HEIGHT;
        let common_path_len = ((leaf_index ^ self.index()) << padding).leading_zeros() as usize;
        (HEIGHT - 1) - common_path_len
    }

    pub fn update_proof(
        &self,
        leaf_index: usize,
        proof: &mut BoundedVec<[u8; 32]>,
    ) -> Result<(), ConcurrentMerkleTreeError> {
        if leaf_index != self.index() {
            let intersection_index = self.intersection_index(leaf_index);
            if let Some(node) = self.path[intersection_index] {
                proof[intersection_index] = node;
            }
        } else {
            // This case means that the leaf we are trying to update was
            // already updated. Therefore, the right thing to do is to notify
            // the caller to sync the local Merkle tree and update the leaf,
            // if necessary.
            return Err(ConcurrentMerkleTreeError::CannotUpdateLeaf);
        }

        Ok(())
    }
}
