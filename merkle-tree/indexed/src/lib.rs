use array::IndexingElement;
use light_concurrent_merkle_tree::ConcurrentMerkleTree;
use light_hasher::{errors::HasherError, Hasher};

pub mod array;
pub mod reference;

#[repr(C)]
pub struct IndexedMerkleTree<
    H,
    const HEIGHT: usize,
    const MAX_CHANGELOG: usize,
    const MAX_ROOTS: usize,
> where
    H: Hasher,
{
    pub merkle_tree: ConcurrentMerkleTree<H, HEIGHT, MAX_CHANGELOG, MAX_ROOTS>,
}

impl<H, const HEIGHT: usize, const MAX_CHANGELOG: usize, const MAX_ROOTS: usize> Default
    for IndexedMerkleTree<H, HEIGHT, MAX_CHANGELOG, MAX_ROOTS>
where
    H: Hasher,
{
    fn default() -> Self {
        Self {
            merkle_tree: ConcurrentMerkleTree::default(),
        }
    }
}

impl<H, const HEIGHT: usize, const MAX_CHANGELOG: usize, const MAX_ROOTS: usize>
    IndexedMerkleTree<H, HEIGHT, MAX_CHANGELOG, MAX_ROOTS>
where
    H: Hasher,
{
    pub fn init(&mut self) -> Result<(), HasherError> {
        self.merkle_tree.init()?;

        // Append the first low leaf, which has value 0 and does not point
        // to any other leaf yet.
        // This low leaf is going to be updated during the first `update`
        // operation.
        self.merkle_tree.append(&H::zero_indexed_leaf())
    }

    pub fn changelog_index(&self) -> usize {
        self.merkle_tree.changelog_index()
    }

    pub fn root_index(&self) -> usize {
        self.merkle_tree.root_index()
    }

    pub fn root(&self) -> Result<[u8; 32], HasherError> {
        self.merkle_tree.root()
    }

    pub fn update(
        &mut self,
        changelog_index: usize,
        new_element: IndexingElement,
        old_low_leaf: &[u8; 32],
        new_low_element: IndexingElement,
        low_leaf_index: usize,
        low_leaf_proof: &[[u8; 32]; HEIGHT],
    ) -> Result<(), HasherError> {
        // Update low element.
        let new_low_leaf = new_low_element.hash::<H>()?;
        self.merkle_tree.update(
            changelog_index,
            &old_low_leaf,
            &new_low_leaf,
            low_leaf_index,
            low_leaf_proof,
        )?;

        // Append new element.
        let new_leaf = new_element.hash::<H>()?;
        self.merkle_tree.append(&new_leaf)?;

        Ok(())
    }
}
