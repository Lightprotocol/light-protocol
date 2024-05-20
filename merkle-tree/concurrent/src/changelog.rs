use light_bounded_vec::BoundedVec;

use crate::errors::ConcurrentMerkleTreeError;

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct ChangelogEntry<const HEIGHT: usize> {
    /// Root.
    pub root: [u8; 32],
    // Path of the changelog.
    pub path: [[u8; 32]; HEIGHT],
    // Index of the affected leaf.
    pub index: u64,
}

pub type ChangelogEntry22 = ChangelogEntry<22>;
pub type ChangelogEntry26 = ChangelogEntry<26>;
pub type ChangelogEntry32 = ChangelogEntry<32>;
pub type ChangelogEntry40 = ChangelogEntry<40>;

impl<const HEIGHT: usize> ChangelogEntry<HEIGHT> {
    pub fn new(root: [u8; 32], path: [[u8; 32]; HEIGHT], index: usize) -> Self {
        let index = index as u64;
        Self { root, path, index }
    }

    pub fn default_with_index(index: usize) -> Self {
        Self {
            root: [0u8; 32],
            path: [[0u8; 32]; HEIGHT],
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
            proof[intersection_index] = self.path[intersection_index];
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

#[cfg(test)]
mod test {
    #[test]
    fn test_get_rightmost_proof() {}
}
