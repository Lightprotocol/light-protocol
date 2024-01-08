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
}
