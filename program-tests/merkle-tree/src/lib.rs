pub mod indexed;

use std::marker::PhantomData;

use light_hasher::{errors::HasherError, Hasher};
use light_indexed_array::errors::IndexedArrayError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ReferenceMerkleTreeError {
    #[error("Leaf {0} does not exist")]
    LeafDoesNotExist(usize),
    #[error("Hasher error: {0}")]
    Hasher(#[from] HasherError),
    #[error("Invalid proof length provided: {0} required {1}")]
    InvalidProofLength(usize, usize),
    #[error("IndexedArray error: {0}")]
    IndexedArray(#[from] IndexedArrayError),
    #[error("RootHistoryArrayLenNotSet")]
    RootHistoryArrayLenNotSet,
}

#[derive(Debug, Clone)]
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
    pub num_root_updates: usize,
    pub sequence_number: usize,
    pub root_history_start_offset: usize,
    pub root_history_array_len: Option<usize>,
    // pub batch_size: Option<usize>,
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
            sequence_number: 0,
            root_history_start_offset: 0,
            root_history_array_len: None,
            num_root_updates: 0,
            _hasher: PhantomData,
        }
    }

    pub fn new_with_history(
        height: usize,
        canopy_depth: usize,
        root_history_start_offset: usize,
        root_history_array_len: usize,
    ) -> Self {
        Self {
            height,
            capacity: 1 << height,
            canopy_depth,
            layers: vec![Vec::new(); height],
            roots: vec![H::zero_bytes()[height]],
            rightmost_index: 0,
            sequence_number: 0,
            root_history_start_offset,
            root_history_array_len: Some(root_history_array_len),
            num_root_updates: 0,
            _hasher: PhantomData,
        }
    }

    pub fn get_history_root_index(&self) -> Result<u16, ReferenceMerkleTreeError> {
        if let Some(root_history_array_len) = self.root_history_array_len {
            Ok(
                ((self.rightmost_index - self.root_history_start_offset) % root_history_array_len)
                    .try_into()
                    .unwrap(),
            )
        } else {
            Err(ReferenceMerkleTreeError::RootHistoryArrayLenNotSet)
        }
    }

    /// Get root history index for v2 (batched) Merkle trees.
    pub fn get_history_root_index_v2(&self) -> Result<u16, ReferenceMerkleTreeError> {
        if let Some(root_history_array_len) = self.root_history_array_len {
            Ok(((self.num_root_updates) % root_history_array_len)
                .try_into()
                .unwrap())
        } else {
            Err(ReferenceMerkleTreeError::RootHistoryArrayLenNotSet)
        }
    }

    /// Number of nodes to include in canopy, based on `canopy_depth`.
    pub fn canopy_size(&self) -> usize {
        (1 << (self.canopy_depth + 1)) - 2
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
        if self.rightmost_index == self.capacity {
            println!("Merkle tree full");
            return Err(HasherError::IntegerOverflow);
        }
        self.rightmost_index += 1;

        self.update_upper_layers(i)?;

        self.sequence_number += 1;
        Ok(())
    }

    pub fn append_batch(&mut self, leaves: &[&[u8; 32]]) -> Result<(), HasherError> {
        for leaf in leaves {
            self.append(leaf)?;
        }
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

        self.sequence_number += 1;
        Ok(())
    }

    pub fn root(&self) -> [u8; 32] {
        // PANICS: We always initialize the Merkle tree with a
        // root (from zero bytes), so the following should never
        // panic.
        self.roots.last().cloned().unwrap()
    }

    pub fn get_path_of_leaf(
        &self,
        mut index: usize,
        full: bool,
    ) -> Result<Vec<[u8; 32]>, ReferenceMerkleTreeError> {
        let mut path = Vec::with_capacity(self.height);
        let limit = match full {
            true => self.height,
            false => self.height - self.canopy_depth,
        };

        for level in 0..limit {
            let node = self.layers[level]
                .get(index)
                .cloned()
                .unwrap_or(H::zero_bytes()[level]);
            path.push(node);

            index /= 2;
        }

        Ok(path)
    }

    pub fn get_proof_of_leaf(
        &self,
        mut index: usize,
        full: bool,
    ) -> Result<Vec<[u8; 32]>, ReferenceMerkleTreeError> {
        let mut proof = Vec::with_capacity(self.height);
        let limit = match full {
            true => self.height,
            false => self.height - self.canopy_depth,
        };

        for level in 0..limit {
            #[allow(clippy::manual_is_multiple_of)]
            let is_left = index % 2 == 0;

            let sibling_index = if is_left { index + 1 } else { index - 1 };
            let node = self.layers[level]
                .get(sibling_index)
                .cloned()
                .unwrap_or(H::zero_bytes()[level]);
            proof.push(node);

            index /= 2;
        }

        Ok(proof)
    }

    pub fn get_proof_by_indices(&self, indices: &[i32]) -> Vec<Vec<[u8; 32]>> {
        let mut proofs = Vec::new();
        for &index in indices {
            let mut index = index as usize;
            let mut proof = Vec::with_capacity(self.height);

            for level in 0..self.height {
                #[allow(clippy::manual_is_multiple_of)]
                let is_left = index % 2 == 0;
                let sibling_index = if is_left { index + 1 } else { index - 1 };
                let node = self.layers[level]
                    .get(sibling_index)
                    .cloned()
                    .unwrap_or(H::zero_bytes()[level]);
                proof.push(node);
                index /= 2;
            }
            proofs.push(proof);
        }
        proofs
    }

    pub fn get_canopy(&self) -> Result<Vec<[u8; 32]>, ReferenceMerkleTreeError> {
        if self.canopy_depth == 0 {
            return Ok(Vec::with_capacity(0));
        }
        let mut canopy = Vec::with_capacity(self.canopy_size());

        let mut num_nodes_in_level = 2;
        for i in 0..self.canopy_depth {
            let level = self.height - 1 - i;
            for j in 0..num_nodes_in_level {
                let node = self.layers[level]
                    .get(j)
                    .cloned()
                    .unwrap_or(H::zero_bytes()[level]);
                canopy.push(node);
            }
            num_nodes_in_level *= 2;
        }

        Ok(canopy)
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

    pub fn leaves(&self) -> &[[u8; 32]] {
        self.layers[0].as_slice()
    }

    pub fn verify(
        &self,
        leaf: &[u8; 32],
        proof: &[[u8; 32]],
        leaf_index: usize,
    ) -> Result<bool, ReferenceMerkleTreeError> {
        if leaf_index >= self.capacity {
            return Err(ReferenceMerkleTreeError::LeafDoesNotExist(leaf_index));
        }
        if proof.len() != self.height {
            return Err(ReferenceMerkleTreeError::InvalidProofLength(
                proof.len(),
                self.height,
            ));
        }

        let mut computed_hash = *leaf;
        let mut current_index = leaf_index;

        for sibling_hash in proof.iter() {
            #[allow(clippy::manual_is_multiple_of)]
            let is_left = current_index % 2 == 0;
            let hashes = if is_left {
                [&computed_hash[..], &sibling_hash[..]]
            } else {
                [&sibling_hash[..], &computed_hash[..]]
            };

            computed_hash = H::hashv(&hashes)?;
            // Move to the parent index for the next iteration
            current_index /= 2;
        }

        // Compare the computed hash to the last known root
        Ok(computed_hash == self.root())
    }

    /// Returns the filled subtrees of the Merkle tree.
    /// Subtrees are the rightmost left node of each level.
    /// Subtrees can be used for efficient append operations.
    pub fn get_subtrees(&self) -> Vec<[u8; 32]> {
        let mut subtrees = H::zero_bytes()[0..self.height].to_vec();
        if self.layers.last().and_then(|layer| layer.first()).is_some() {
            for level in (0..self.height).rev() {
                if let Some(left_child) = self.layers.get(level).and_then(|layer| {
                    if layer.len() % 2 == 0 {
                        layer.get(layer.len() - 2)
                    } else {
                        layer.last()
                    }
                }) {
                    subtrees[level] = *left_child;
                }
            }
        }
        subtrees
    }

    pub fn get_next_index(&self) -> usize {
        self.rightmost_index + 1
    }

    pub fn get_leaf(&self, index: usize) -> Result<[u8; 32], ReferenceMerkleTreeError> {
        self.layers[0]
            .get(index)
            .cloned()
            .ok_or(ReferenceMerkleTreeError::LeafDoesNotExist(index))
    }
}

#[cfg(test)]
mod tests {
    use light_hasher::{zero_bytes::poseidon::ZERO_BYTES, Poseidon};

    use super::*;

    const TREE_AFTER_1_UPDATE: [[u8; 32]; 4] = [
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1,
        ],
        [
            0, 122, 243, 70, 226, 211, 4, 39, 158, 121, 224, 169, 243, 2, 63, 119, 18, 148, 167,
            138, 203, 112, 231, 63, 144, 175, 226, 124, 173, 64, 30, 129,
        ],
        [
            4, 163, 62, 195, 162, 201, 237, 49, 131, 153, 66, 155, 106, 112, 192, 40, 76, 131, 230,
            239, 224, 130, 106, 36, 128, 57, 172, 107, 60, 247, 103, 194,
        ],
        [
            7, 118, 172, 114, 242, 52, 137, 62, 111, 106, 113, 139, 123, 161, 39, 255, 86, 13, 105,
            167, 223, 52, 15, 29, 137, 37, 106, 178, 49, 44, 226, 75,
        ],
    ];

    const TREE_AFTER_2_UPDATES: [[u8; 32]; 4] = [
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 2,
        ],
        [
            0, 122, 243, 70, 226, 211, 4, 39, 158, 121, 224, 169, 243, 2, 63, 119, 18, 148, 167,
            138, 203, 112, 231, 63, 144, 175, 226, 124, 173, 64, 30, 129,
        ],
        [
            18, 102, 129, 25, 152, 42, 192, 218, 100, 215, 169, 202, 77, 24, 100, 133, 45, 152, 17,
            121, 103, 9, 187, 226, 182, 36, 35, 35, 126, 255, 244, 140,
        ],
        [
            11, 230, 92, 56, 65, 91, 231, 137, 40, 92, 11, 193, 90, 225, 123, 79, 82, 17, 212, 147,
            43, 41, 126, 223, 49, 2, 139, 211, 249, 138, 7, 12,
        ],
    ];

    #[test]
    fn test_subtrees() {
        let tree_depth = 4;
        let mut tree = MerkleTree::<Poseidon>::new(tree_depth, 0);

        let subtrees = tree.get_subtrees();
        for (i, subtree) in subtrees.iter().enumerate() {
            assert_eq!(*subtree, ZERO_BYTES[i]);
        }

        let mut leaf_0: [u8; 32] = [0; 32];
        leaf_0[31] = 1;
        tree.append(&leaf_0).unwrap();
        tree.append(&leaf_0).unwrap();

        let subtrees = tree.get_subtrees();
        for (i, subtree) in subtrees.iter().enumerate() {
            assert_eq!(*subtree, TREE_AFTER_1_UPDATE[i]);
        }

        let mut leaf_1: [u8; 32] = [0; 32];
        leaf_1[31] = 2;
        tree.append(&leaf_1).unwrap();
        tree.append(&leaf_1).unwrap();

        let subtrees = tree.get_subtrees();
        for (i, subtree) in subtrees.iter().enumerate() {
            assert_eq!(*subtree, TREE_AFTER_2_UPDATES[i]);
        }
    }
}
