#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct ChangelogEntry<const MAX_HEIGHT: usize> {
    /// Root.
    pub root: [u8; 32],
    // Path of the changelog.
    pub path: [[u8; 32]; MAX_HEIGHT],
    // Index.
    pub index: u64,
}

impl<const MAX_HEIGHT: usize> Default for ChangelogEntry<MAX_HEIGHT> {
    fn default() -> Self {
        Self {
            root: [0u8; 32],
            path: [[0u8; 32]; MAX_HEIGHT],
            index: 0,
        }
    }
}

impl<const MAX_HEIGHT: usize> ChangelogEntry<MAX_HEIGHT> {
    pub fn new(root: [u8; 32], path: [[u8; 32]; MAX_HEIGHT], index: usize) -> Self {
        let index = index as u64;
        Self { root, path, index }
    }
}
