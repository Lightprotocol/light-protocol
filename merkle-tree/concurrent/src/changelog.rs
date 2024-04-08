use light_bounded_vec::{BoundedVec, Pod};

use crate::errors::ConcurrentMerkleTreeError;

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct ChangelogEntry<const HEIGHT: usize> {
    /// Root.
    pub root: [u8; 32],
    // Path of the changelog.
    pub path: [[u8; 32]; HEIGHT],
    // Index.
    pub index: u64,
    // Sequence number.
    pub seq: u64,
}

pub type ChangelogEntry22 = ChangelogEntry<22>;
pub type ChangelogEntry26 = ChangelogEntry<26>;
pub type ChangelogEntry32 = ChangelogEntry<32>;
pub type ChangelogEntry40 = ChangelogEntry<40>;

unsafe impl<const HEIGHT: usize> Pod for ChangelogEntry<HEIGHT> {}

impl<const HEIGHT: usize> ChangelogEntry<HEIGHT> {
    pub fn new(root: [u8; 32], path: [[u8; 32]; HEIGHT], index: usize, seq: usize) -> Self {
        let index = index as u64;
        let seq = seq as u64;
        Self {
            root,
            path,
            index,
            seq,
        }
    }

    pub fn default_with_index(index: usize) -> Self {
        Self {
            root: [0u8; 32],
            path: [[0u8; 32]; HEIGHT],
            index: index as u64,
            seq: 0,
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
            // already updated. Therefore, updating the proof is impossible.
            // We need to return an error and request the caller
            // to retry the update with a new proof.
            return Err(ConcurrentMerkleTreeError::CannotUpdateLeaf);
        }

        Ok(())
    }

    pub fn update_subtrees(&self, rightmost_index: usize, subtrees: &mut BoundedVec<[u8; 32]>) {
        let (mut current_index, start) = if rightmost_index != self.index() {
            let intersection_index = self.intersection_index(rightmost_index);
            let current_index = rightmost_index + intersection_index;

            subtrees[intersection_index] = self.path[intersection_index];

            (current_index, intersection_index)
        } else {
            (rightmost_index, 0)
        };

        for (i, subtree) in subtrees.iter_mut().enumerate().skip(start) {
            let is_left = current_index % 2 == 0;
            if is_left {
                *subtree = self.path[i];
            }

            current_index /= 2;
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_get_rightmost_proof() {}
}
