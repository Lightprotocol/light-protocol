use light_hasher::Poseidon;
use light_merkle_tree_reference::MerkleTree;
use tracing::debug;

use crate::error::ForesterUtilsError;

pub const TREE_HEIGHT: usize = 32;

/// Result of a batch update operation on a staging tree.
#[derive(Clone, Debug)]
pub struct BatchUpdateResult {
    pub old_leaves: Vec<[u8; 32]>,
    pub merkle_proofs: Vec<Vec<[u8; 32]>>,
    pub old_root: [u8; 32],
    pub new_root: [u8; 32],
}

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

    fn ensure_layer_capacity(&mut self, level: usize, min_index: usize, context: &str) {
        if level < self.tree.layers.len() && self.tree.layers[level].len() <= min_index {
            let old_len = self.tree.layers[level].len();
            self.tree.ensure_layer_capacity(level, min_index);
            debug!(
                "Auto-expanded tree layer {}: {} -> {} nodes ({})",
                level,
                old_len,
                self.tree.layers[level].len(),
                context
            );
        }
    }

    fn do_tree_update(
        &mut self,
        leaf_index: u64,
        new_leaf: [u8; 32],
    ) -> Result<(), ForesterUtilsError> {
        self.tree
            .update(&new_leaf, leaf_index as usize)
            .map_err(|e| {
                ForesterUtilsError::StagingTree(format!(
                    "Failed to update leaf {}: {:?}",
                    leaf_index, e
                ))
            })
    }

    pub fn update_leaf(
        &mut self,
        leaf_index: u64,
        new_leaf: [u8; 32],
    ) -> Result<(), ForesterUtilsError> {
        let leaf_idx = leaf_index as usize;
        self.ensure_layer_capacity(0, leaf_idx, &format!("for index {}", leaf_idx));
        self.do_tree_update(leaf_index, new_leaf)?;
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
    ) -> Result<BatchUpdateResult, ForesterUtilsError> {
        if leaf_indices.len() != new_leaves.len() {
            return Err(ForesterUtilsError::StagingTree(format!(
                "Mismatch: {} leaf indices but {} new leaves",
                leaf_indices.len(),
                new_leaves.len()
            )));
        }

        let old_root = self.current_root();

        if let Some(&max_leaf_idx) = leaf_indices.iter().max() {
            self.ensure_layer_capacity(
                0,
                max_leaf_idx as usize,
                &format!(
                    "{} batch {} max index {}",
                    batch_type, batch_idx, max_leaf_idx
                ),
            );
        }

        let mut old_leaves = Vec::with_capacity(leaf_indices.len());
        let mut merkle_proofs = Vec::with_capacity(leaf_indices.len());

        for (&leaf_idx, &new_leaf) in leaf_indices.iter().zip(new_leaves.iter()) {
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

            self.do_tree_update(leaf_idx, final_leaf)?;
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

        Ok(BatchUpdateResult {
            old_leaves,
            merkle_proofs,
            old_root,
            new_root,
        })
    }

    pub fn get_proof(&self, leaf_index: u64) -> Result<Vec<[u8; 32]>, ForesterUtilsError> {
        self.tree
            .get_proof_of_leaf(leaf_index as usize, true)
            .map_err(|e| ForesterUtilsError::StagingTree(format!("Failed to get proof: {}", e)))
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
        for (&node_index, &node_hash) in nodes.iter().zip(node_hashes.iter()) {
            tree.insert_node(node_index, node_hash).map_err(|e| {
                ForesterUtilsError::StagingTree(format!("Failed to insert node: {}", e))
            })?;
        }

        for (&leaf_index, &leaf_hash) in leaf_indices.iter().zip(leaves.iter()) {
            tree.insert_leaf(leaf_index as usize, leaf_hash);
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
