use light_hasher::Poseidon;
use light_merkle_tree_reference::MerkleTree;
use light_prover_client::proof_types::batch_update::BatchTreeUpdateResult;
use tracing::debug;

use crate::error::ForesterUtilsError;

/// Result of a batch update operation on a staging tree.
#[derive(Clone, Debug)]
pub struct BatchUpdateResult {
    pub old_leaves: Vec<[u8; 32]>,
    pub merkle_proofs: Vec<Vec<[u8; 32]>>,
    pub old_root: [u8; 32],
    pub new_root: [u8; 32],
}

impl From<BatchUpdateResult> for BatchTreeUpdateResult {
    fn from(result: BatchUpdateResult) -> Self {
        BatchTreeUpdateResult {
            old_leaves: result.old_leaves,
            merkle_proofs: result.merkle_proofs,
            old_root: result.old_root,
            new_root: result.new_root,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchType {
    Append,
    Nullify,
}

impl std::fmt::Display for BatchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchType::Append => write!(f, "APPEND"),
            BatchType::Nullify => write!(f, "NULLIFY"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StagingTree {
    tree: MerkleTree<Poseidon>,
    current_root: [u8; 32],
    /// Updates tracked as (leaf_index, new_leaf, batch_seq)
    updates: Vec<(u64, [u8; 32], u64)>,
    base_seq: u64,
}

impl StagingTree {
    pub fn current_root(&self) -> [u8; 32] {
        self.current_root
    }

    pub fn base_seq(&self) -> u64 {
        self.base_seq
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
        batch_seq: u64,
    ) -> Result<(), ForesterUtilsError> {
        let leaf_idx = leaf_index as usize;
        self.ensure_layer_capacity(0, leaf_idx, &format!("for index {}", leaf_idx));
        self.do_tree_update(leaf_index, new_leaf)?;
        self.updates.push((leaf_index, new_leaf, batch_seq));
        self.current_root = self.tree.root();
        Ok(())
    }

    pub fn process_batch_updates(
        &mut self,
        leaf_indices: &[u64],
        new_leaves: &[[u8; 32]],
        batch_type: BatchType,
        batch_idx: usize,
        batch_seq: u64,
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

            let final_leaf = match batch_type {
                BatchType::Nullify => new_leaf,
                BatchType::Append => {
                    let is_old_leaf_zero = old_leaf.iter().all(|&byte| byte == 0);
                    if is_old_leaf_zero {
                        new_leaf
                    } else {
                        old_leaf
                    }
                }
            };

            self.do_tree_update(leaf_idx, final_leaf)?;
            self.updates.push((leaf_idx, final_leaf, batch_seq));

            merkle_proofs.push(proof);
        }

        let new_root = self.tree.root();
        self.current_root = new_root;

        debug!(
            "{} batch {} root transition: {:?}[..4] -> {:?}[..4]",
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

    pub fn get_updates(&self) -> &[(u64, [u8; 32], u64)] {
        &self.updates
    }

    pub fn clear_updates(&mut self) {
        self.updates.clear();
    }

    pub fn new(
        leaf_indices: &[u64],
        leaves: &[[u8; 32]],
        nodes: &[u64],
        node_hashes: &[[u8; 32]],
        initial_root: [u8; 32],
        root_seq: u64,
        height: usize,
    ) -> Result<Self, ForesterUtilsError> {
        debug!(
            "StagingTree::new: {} leaves, {} deduplicated nodes, initial_root={:?}, root_seq={}, height={}",
            leaves.len(),
            nodes.len(),
            &initial_root,
            root_seq,
            height
        );
        let mut tree = MerkleTree::<Poseidon>::new(height, 0);
        for (&node_index, &node_hash) in nodes.iter().zip(node_hashes.iter()) {
            // Skip nodes at root level - root is stored separately in tree.roots
            let level = (node_index >> 56) as usize;
            if level >= height {
                continue;
            }
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
            base_seq: root_seq,
        })
    }

    pub fn replay_pending_updates(
        &mut self,
        pending_updates: &[(u64, [u8; 32], u64)],
        indexer_seq: u64,
    ) -> (usize, usize, usize) {
        let total = pending_updates.len();
        let mut replayed = 0;
        let mut skipped_confirmed = 0;

        for &(leaf_idx, new_leaf, update_seq) in pending_updates {
            // Skip updates from batches that have already been confirmed on-chain
            if update_seq <= indexer_seq {
                skipped_confirmed += 1;
                continue;
            }

            let current_leaf = self.get_leaf(leaf_idx);
            let is_zero = current_leaf.iter().all(|&b| b == 0);

            if is_zero {
                let leaf_idx_usize = leaf_idx as usize;
                self.ensure_layer_capacity(0, leaf_idx_usize, "replay pending");
                if self.do_tree_update(leaf_idx, new_leaf).is_ok() {
                    self.updates.push((leaf_idx, new_leaf, update_seq));
                    replayed += 1;
                }
            }
        }

        if replayed > 0 {
            self.current_root = self.tree.root();
        }

        (total, replayed, skipped_confirmed)
    }
}
