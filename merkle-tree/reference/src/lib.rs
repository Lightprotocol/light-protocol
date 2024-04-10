use std::marker::PhantomData;

use light_bounded_vec::{BoundedVec, BoundedVecError};
use light_hasher::{errors::HasherError, Hasher};
use thiserror::Error;

pub mod store;

#[derive(Debug, Error)]
pub enum ReferenceMerkleTreeError {
    #[error("Leaf {0} does not exist")]
    LeafDoesNotExist(usize),
    #[error("Hasher error: {0}")]
    Hasher(#[from] HasherError),
}

#[derive(Debug)]
pub struct MerkleTree<H>
where
    H: Hasher,
{
    pub height: usize,
    pub capacity: usize,
    pub canopy_depth: usize,
    pub layers: Vec<Vec<[u8; 32]>>,
    pub roots: Vec<[u8; 32]>,
    pub rightmost_index: usize,

    _hasher: PhantomData<H>,
}

impl<H> MerkleTree<H>
where
    H: Hasher,
{
    pub fn new(height: usize, canopy_depth: usize) -> Self {
        Self {
            height,
            capacity: 1 << height,
            canopy_depth,
            layers: vec![Vec::new(); height],
            roots: vec![H::zero_bytes()[height]],
            rightmost_index: 0,

            _hasher: PhantomData,
        }
    }

    fn update_upper_layers(&mut self, mut i: usize) -> Result<(), HasherError> {
        for level in 1..self.height {
            i /= 2;

            let left_index = i * 2;
            let right_index = i * 2 + 1;

            let left_child = self.layers[level - 1]
                .get(left_index)
                .cloned()
                .unwrap_or(H::zero_bytes()[level - 1]);
            let right_child = self.layers[level - 1]
                .get(right_index)
                .cloned()
                .unwrap_or(H::zero_bytes()[level - 1]);

            let node = H::hashv(&[&left_child[..], &right_child[..]])?;
            if self.layers[level].len() > i {
                // A node already exists and we are overwriting it.
                self.layers[level][i] = node;
            } else {
                // A node didn't exist before.
                self.layers[level].push(node);
            }
        }

        let left_child = &self.layers[self.height - 1]
            .first()
            .cloned()
            .unwrap_or(H::zero_bytes()[self.height - 1]);
        let right_child = &self.layers[self.height - 1]
            .get(1)
            .cloned()
            .unwrap_or(H::zero_bytes()[self.height - 1]);
        let root = H::hashv(&[&left_child[..], &right_child[..]])?;

        self.roots.push(root);

        Ok(())
    }

    pub fn append(&mut self, leaf: &[u8; 32]) -> Result<(), HasherError> {
        self.layers[0].push(*leaf);

        let i = self.rightmost_index;
        self.rightmost_index += 1;

        self.update_upper_layers(i)?;

        Ok(())
    }

    pub fn update(
        &mut self,
        leaf: &[u8; 32],
        leaf_index: usize,
    ) -> Result<(), ReferenceMerkleTreeError> {
        *self.layers[0]
            .get_mut(leaf_index)
            .ok_or(ReferenceMerkleTreeError::LeafDoesNotExist(leaf_index))? = *leaf;

        self.update_upper_layers(leaf_index)?;

        Ok(())
    }

    pub fn root(&self) -> [u8; 32] {
        // PANICS: We always initialize the Merkle tree with a
        // root (from zero bytes), so the following should never
        // panic.
        self.roots.last().cloned().unwrap()
    }

    pub fn get_proof_of_leaf(
        &self,
        mut index: usize,
        full: bool,
    ) -> Result<BoundedVec<[u8; 32]>, BoundedVecError> {
        let mut proof = BoundedVec::with_capacity(self.height);
        let limit = match full {
            true => self.height,
            false => self.height - self.canopy_depth,
        };

        for level in 0..limit {
            let is_left = index % 2 == 0;

            let sibling_index = if is_left { index + 1 } else { index - 1 };
            let node = self.layers[level]
                .get(sibling_index)
                .cloned()
                .unwrap_or(H::zero_bytes()[level]);
            proof.push(node)?;

            index /= 2;
        }

        Ok(proof)
    }

    pub fn leaf(&self, leaf_index: usize) -> [u8; 32] {
        self.layers[0]
            .get(leaf_index)
            .cloned()
            .unwrap_or(H::zero_bytes()[0])
    }

    pub fn get_leaf_index(&self, leaf: &[u8; 32]) -> Option<usize> {
        self.layers[0].iter().position(|node| node == leaf)
    }
}
