use crate::error::ForesterUtilsError;
use light_hasher::Poseidon;
use light_merkle_tree_reference::MerkleTree;
use tracing::{debug, warn};

pub const TREE_HEIGHT: usize = 32;

#[derive(Clone, Debug)]
pub struct StagingTree {
    tree: MerkleTree<Poseidon>,
    current_root: [u8; 32],
    updates: Vec<(u64, [u8; 32])>,
}

impl StagingTree {
    pub fn current_root(&self) -> [u8; 32] {
        self.current_root
    }

    pub fn get_leaf(&self, leaf_index: u64) -> [u8; 32] {
        self.tree.layers[0]
            .get(leaf_index as usize)
            .copied()
            .unwrap_or([0u8; 32])
    }

    pub fn update_leaf(
        &mut self,
        leaf_index: u64,
        new_leaf: [u8; 32],
    ) -> Result<(), ForesterUtilsError> {
        let leaf_idx = leaf_index as usize;

        if self.tree.layers[0].len() <= leaf_idx {
            let old_len = self.tree.layers[0].len();
            self.tree.layers[0].resize(leaf_idx + 1, [0u8; 32]);
            debug!(
                "Auto-expanded tree layer 0: {} -> {} leaves (for index {})",
                old_len,
                self.tree.layers[0].len(),
                leaf_idx
            );
        }

        self.tree.update(&new_leaf, leaf_idx).map_err(|e| {
            ForesterUtilsError::StagingTree(format!(
                "Failed to update leaf {}: {:?}",
                leaf_index, e
            ))
        })?;
        self.updates.push((leaf_index, new_leaf));
        self.current_root = self.tree.root();
        Ok(())
    }

    pub fn process_batch_updates(
        &mut self,
        leaf_indices: &[u64],
        new_leaves: &[[u8; 32]],
        batch_type: &str,
        batch_idx: usize,
    ) -> Result<(Vec<[u8; 32]>, Vec<Vec<[u8; 32]>>, [u8; 32], [u8; 32]), ForesterUtilsError> {
        use light_hasher::Hasher;

        if leaf_indices.len() != new_leaves.len() {
            return Err(ForesterUtilsError::StagingTree(format!(
                "Mismatch: {} leaf indices but {} new leaves",
                leaf_indices.len(),
                new_leaves.len()
            )));
        }

        let old_root = self.current_root();

        // Pre-expand tree to accommodate all leaf indices
        if let Some(&max_leaf_idx) = leaf_indices.iter().max() {
            let max_idx = max_leaf_idx as usize;
            if self.tree.layers[0].len() <= max_idx {
                let old_len = self.tree.layers[0].len();
                self.tree.layers[0].resize(max_idx + 1, [0u8; 32]);
                debug!(
                    "Pre-expanded tree for {} batch {}: {} -> {} leaves (max index in batch: {})",
                    batch_type,
                    batch_idx,
                    old_len,
                    self.tree.layers[0].len(),
                    max_idx
                );
            }
        }

        let mut old_leaves = Vec::with_capacity(leaf_indices.len());
        let mut merkle_proofs = Vec::with_capacity(leaf_indices.len());

        for (i, (&leaf_idx, &new_leaf)) in leaf_indices.iter().zip(new_leaves.iter()).enumerate() {
            let old_leaf = self.get_leaf(leaf_idx);
            let proof = self.get_proof(leaf_idx)?;
            old_leaves.push(old_leaf);

            let final_leaf = if batch_type == "NULLIFY" {
                new_leaf
            } else {
                let is_old_leaf_zero = old_leaf.iter().all(|&byte| byte == 0);
                if is_old_leaf_zero {
                    new_leaf
                } else {
                    old_leaf
                }
            };

            self.tree.update(&final_leaf, leaf_idx as usize).map_err(|e| {
                ForesterUtilsError::StagingTree(format!(
                    "Failed to update leaf {}: {:?}",
                    leaf_idx, e
                ))
            })?;
            self.updates.push((leaf_idx, final_leaf));

            merkle_proofs.push(proof);
        }

        let new_root = self.tree.root();
        self.current_root = new_root;

        debug!(
            "   {} batch {} root transition: {:?}[..4] -> {:?}[..4]",
            batch_type,
            batch_idx,
            &old_root[..4],
            &new_root[..4]
        );

        Ok((old_leaves, merkle_proofs, old_root, new_root))
    }

    pub fn get_proof(&self, leaf_index: u64) -> Result<Vec<[u8; 32]>, ForesterUtilsError> {
        let mut proof = Vec::with_capacity(TREE_HEIGHT);
        let mut current_index = leaf_index;

        for level in 0..(TREE_HEIGHT as u8) {
            let level_usize = level as usize;

            let sibling_position = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            let sibling = if level_usize < self.tree.layers.len() {
                let layer_val = self.tree.layers[level_usize]
                    .get(sibling_position as usize)
                    .copied()
                    .unwrap_or([0u8; 32]);
                if leaf_index == 0 && level < 3 {
                    debug!(
                        "get_proof leaf={} level={} sibling_pos={} layer_size={} value={:?}",
                        leaf_index, level, sibling_position, self.tree.layers[level_usize].len(), &layer_val[..8]
                    );
                }
                layer_val
            } else {
                [0u8; 32]
            };

            proof.push(sibling);
            current_index /= 2;
        }

        Ok(proof)
    }

    pub fn get_updates(&self) -> &[(u64, [u8; 32])] {
        &self.updates
    }

    pub fn clear_updates(&mut self) {
        self.updates.clear();
    }

    pub fn into_updates(self) -> Vec<(u64, [u8; 32])> {
        self.updates
    }

    pub fn from_indexer_elements(
        elements: &[light_client::indexer::MerkleProofWithContext],
    ) -> Result<Self, ForesterUtilsError> {
        let mut tree = MerkleTree::<Poseidon>::new(TREE_HEIGHT, 0);

        for element in elements {
            let leaf_idx = element.leaf_index as usize;

            if tree.layers[0].len() <= leaf_idx {
                tree.layers[0].resize(leaf_idx + 1, [0u8; 32]);
            }

            tree.layers[0][leaf_idx] = element.leaf;

            let proof = &element.proof;
            let mut current_idx = leaf_idx;

            for (level, proof_node) in proof.iter().enumerate() {
                let next_level = level + 1;
                if next_level >= tree.layers.len() {
                    break;
                }

                let required_size = (current_idx / 2) + 1;
                if tree.layers[next_level].len() < required_size {
                    tree.layers[next_level].resize(required_size, [0u8; 32]);
                }

                let sibling_idx = current_idx ^ 1;
                if tree.layers[level].len() <= sibling_idx {
                    tree.layers[level].resize(sibling_idx + 1, [0u8; 32]);
                }
                tree.layers[level][sibling_idx] = *proof_node;

                current_idx /= 2;
            }
        }

        let computed_root = tree.root();

        Ok(Self {
            tree,
            current_root: computed_root,
            updates: Vec::new(),
        })
    }

    pub fn from_v2_output_queue(
        leaf_indices: &[u64],
        leaves: &[[u8; 32]],
        nodes: &[u64],
        node_hashes: &[[u8; 32]],
        initial_root: [u8; 32],
    ) -> Result<Self, ForesterUtilsError> {
        debug!(
            "from_v2_output_queue: {} leaves, {} deduplicated nodes, initial_root={:?}",
            leaves.len(),
            nodes.len(),
            &initial_root
        );
        let mut tree = MerkleTree::<Poseidon>::new(TREE_HEIGHT, 0);
        for (node_index, node_hash) in nodes.iter().zip(node_hashes.iter()) {
            let level = (node_index >> 56) as usize;
            let position = (node_index & 0x00FFFFFFFFFFFFFF) as usize;

            if level >= tree.layers.len() {
                debug!(
                    "Skipping node at level {} (position {}) - exceeds tree height {}",
                    level,
                    position,
                    tree.layers.len()
                );
                continue;
            }

            if tree.layers[level].len() <= position {
                tree.layers[level].resize(position + 1, [0u8; 32]);
                debug!(
                    "Auto-expanded tree layer {}: -> {} nodes (for position {})",
                    level,
                    tree.layers[level].len(),
                    position
                );
            }

            tree.layers[level][position] = *node_hash;
        }

        for (leaf_index, leaf_hash) in leaf_indices.iter().zip(leaves.iter()) {
            let leaf_idx = *leaf_index as usize;

            if tree.layers[0].len() <= leaf_idx {
                tree.layers[0].resize(leaf_idx + 1, [0u8; 32]);
            }

            // Store the leaf
            tree.layers[0][leaf_idx] = *leaf_hash;
        }
        tree.roots.push(initial_root);

        Ok(Self {
            tree,
            current_root: initial_root,
            updates: Vec::new(),
        })
    }

    pub fn from_v2_input_queue(
        leaf_indices: &[u64],
        leaves: &[[u8; 32]],
        nodes: &[u64],
        node_hashes: &[[u8; 32]],
        initial_root: [u8; 32],
    ) -> Result<Self, ForesterUtilsError> {
        Self::from_v2_output_queue(leaf_indices, leaves, nodes, node_hashes, initial_root)
    }
}