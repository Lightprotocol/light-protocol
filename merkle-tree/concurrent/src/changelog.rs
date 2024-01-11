#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
pub struct ChangelogEntry<const HEIGHT: usize> {
    /// Root.
    pub root: [u8; 32],
    // Path of the changelog.
    pub path: [[u8; 32]; HEIGHT],
    // Index.
    pub index: u64,
}

impl<const HEIGHT: usize> Default for ChangelogEntry<HEIGHT> {
    fn default() -> Self {
        Self {
            root: [0u8; 32],
            path: [[0u8; 32]; HEIGHT],
            index: 0,
        }
    }
}

impl<const HEIGHT: usize> ChangelogEntry<HEIGHT> {
    pub fn new(root: [u8; 32], path: [[u8; 32]; HEIGHT], index: usize) -> Self {
        let index = index as u64;
        Self { root, path, index }
    }

    /// Returns an intersection index in the changelog entry which affects the
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
    fn intersection_index(&self, leaf_index: usize, changelog_entry_index: usize) -> usize {
        let padding = 64 - HEIGHT;
        let common_path_len =
            ((leaf_index ^ changelog_entry_index) << padding).leading_zeros() as usize;

        (HEIGHT - 1) - common_path_len
    }

    pub fn update_proof(
        &self,
        leaf_index: usize,
        proof: &[[u8; 32]; HEIGHT],
    ) -> Option<[[u8; 32]; HEIGHT]> {
        let mut updated_proof = proof.to_owned();

        let changelog_entry_index = self.index as usize;
        if leaf_index != changelog_entry_index {
            let intersection_index = self.intersection_index(leaf_index, changelog_entry_index);
            updated_proof[intersection_index] = self.path[intersection_index];
        } else {
            return None;
        }

        Some(updated_proof)
    }
}
