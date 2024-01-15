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
        new_element_next_value: [u8; 32],
        low_element: IndexingElement,
        low_element_next_value: [u8; 32],
        low_leaf_proof: &[[u8; 32]; HEIGHT],
    ) -> Result<(), HasherError> {
        // Check that the value of `new_element` belongs to the range
        // of `old_low_element`.
        if low_element.next_index == 0 {
            // In this case, the `old_low_element` is the greatest element.
            // The value of `new_element` needs to be greater than the value of
            // `old_low_element` (and therefore, be the greatest).
            if new_element.value <= low_element.value {
                return Err(HasherError::LowElementGreaterOrEqualToNewElement);
            }
        } else {
            // The value of `new_element` needs to be greater than the value of
            // `old_low_element` (and therefore, be the greatest).
            if new_element.value <= low_element.value {
                return Err(HasherError::LowElementGreaterOrEqualToNewElement);
            }
            // The value of `new_element` needs to be lower than the value of
            // next element pointed by `old_low_element`.
            if new_element.value >= low_element_next_value {
                return Err(HasherError::NewElementGreaterOrEqualToNextElement);
            }
        }

        // Instantiate `new_low_element` - the low element with updated values.
        let new_low_element = IndexingElement {
            index: low_element.index,
            value: low_element.value,
            next_index: new_element.index,
        };

        // Update low element. If the `old_low_element` does not belong to the
        // tree, validating the proof is going to fail.
        let old_low_leaf = low_element.hash::<H>(low_element_next_value)?;
        let new_low_leaf = new_low_element.hash::<H>(new_element.value)?;
        self.merkle_tree.update(
            changelog_index,
            &old_low_leaf,
            &new_low_leaf,
            low_element.index.into(),
            low_leaf_proof,
        )?;

        // Append new element.
        let new_leaf = new_element.hash::<H>(new_element_next_value)?;
        self.merkle_tree.append(&new_leaf)?;

        Ok(())
    }
}
