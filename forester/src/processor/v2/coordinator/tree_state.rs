use anyhow::{anyhow, Result};
use light_client::indexer::{InputQueueDataV2, OutputQueueDataV2};
use light_hasher::Poseidon;
use light_merkle_tree_reference::MerkleTree;
use tracing::{debug, trace};

pub const TREE_HEIGHT: usize = 32;

#[derive(Debug, Clone)]
pub struct TreeState {
    tree: MerkleTree<Poseidon>,
    cached_root: [u8; 32],
    root_dirty: bool,
}

/// Staging tree for incremental proof generation.
///
/// Maintains a working copy of the tree that can be updated incrementally.
/// Each call to get_proof returns a proof that includes all prior updates,
/// eliminating the need for changelog adjustments.
#[derive(Clone)]
pub struct StagingTree {
    tree: MerkleTree<Poseidon>,
    base_root: [u8; 32],
    updates: Vec<(u64, [u8; 32])>,
}

impl StagingTree {
    pub fn base_root(&self) -> [u8; 32] {
        self.base_root
    }

    pub fn current_root(&self) -> [u8; 32] {
        self.tree.root()
    }

    pub fn get_leaf(&self, leaf_index: u64) -> [u8; 32] {
        self.tree.layers[0]
            .get(leaf_index as usize)
            .copied()
            .unwrap_or([0u8; 32])
    }

    pub fn update_leaf(&mut self, leaf_index: u64, new_leaf: [u8; 32]) -> Result<()> {
        let leaf_idx = leaf_index as usize;

        // Auto-expand tree layers if needed to accommodate this leaf index
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

        self.tree
            .update(&new_leaf, leaf_idx)
            .map_err(|e| anyhow!("Failed to update leaf {}: {:?}", leaf_index, e))?;
        self.updates.push((leaf_index, new_leaf));
        Ok(())
    }

    pub fn get_proof(&self, leaf_index: u64) -> Result<Vec<[u8; 32]>> {
        let leaf_idx = leaf_index as usize;

        // Check if leaf exists, if not we need to expand the tree first
        if self.tree.layers[0].len() <= leaf_idx {
            // This shouldn't happen in normal flow, but handle gracefully
            return Err(anyhow!(
                "Cannot get proof for leaf {} - leaf does not exist (tree has {} leaves)",
                leaf_index,
                self.tree.layers[0].len()
            ));
        }

        self.tree
            .get_proof_of_leaf(leaf_idx, true)
            .map_err(|e| anyhow!("Failed to get proof for leaf {}: {:?}", leaf_index, e))
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

    /// Construct a StagingTree from indexer data.
    /// Initializes a dense tree structure to support proof generation.
    pub fn from_v2_response(
        output_queue: Option<&OutputQueueDataV2>,
        input_queue: Option<&InputQueueDataV2>,
    ) -> Result<Self> {
        let mut tree = MerkleTree::<Poseidon>::new(TREE_HEIGHT, 0);
        let mut base_root = [0u8; 32];

        // Load deduplicated nodes from output queue (APPEND operations)
        if let Some(oq) = output_queue {
            debug!(
                "StagingTree: Loading {} deduplicated nodes from output queue",
                oq.nodes.len()
            );

            for (node_idx, node_hash) in oq.nodes.iter().zip(oq.node_hashes.iter()) {
                let (level, position) = decode_node_index(*node_idx);
                Self::set_node_in_tree(&mut tree, level as usize, position as usize, *node_hash)?;
            }

            for (leaf_idx, leaf_hash) in oq.leaf_indices.iter().zip(oq.old_leaves.iter()) {
                Self::set_node_in_tree(&mut tree, 0, *leaf_idx as usize, *leaf_hash)?;
            }

            base_root = oq.initial_root;
        }

        // Load deduplicated nodes from input queue (NULLIFY operations)
        if let Some(iq) = input_queue {
            debug!(
                "StagingTree: Loading {} deduplicated nodes from input queue",
                iq.nodes.len()
            );

            for (node_idx, node_hash) in iq.nodes.iter().zip(iq.node_hashes.iter()) {
                let (level, position) = decode_node_index(*node_idx);
                Self::set_node_in_tree(&mut tree, level as usize, position as usize, *node_hash)?;
            }

            for (leaf_idx, leaf_hash) in iq.leaf_indices.iter().zip(iq.current_leaves.iter()) {
                Self::set_node_in_tree(&mut tree, 0, *leaf_idx as usize, *leaf_hash)?;
            }

            if output_queue.is_none() {
                base_root = iq.initial_root;
            }
        }

        // NOTE: We do NOT verify the tree root here because the tree is loaded with
        // deduplicated nodes from the indexer. The tree structure is sparse but sufficient
        // for incremental proof generation via the staging tree mechanism.
        debug!(
            "StagingTree loaded: base_root={:?}, nodes={} total (sparse, deduplicated)",
            &base_root[..8],
            tree.layers.iter().map(|l| l.len()).sum::<usize>()
        );

        Ok(Self {
            tree,
            base_root,
            updates: Vec::new(),
        })
    }

    fn set_node_in_tree(
        tree: &mut MerkleTree<Poseidon>,
        level: usize,
        position: usize,
        hash: [u8; 32],
    ) -> Result<()> {
        if tree.layers[level].len() <= position {
            tree.layers[level].resize(position + 1, [0u8; 32]);
        }
        tree.layers[level][position] = hash;
        Ok(())
    }

    /// Merge fresh indexer data into this cached staging tree.
    pub fn merge_fresh_nodes_from_indexer(
        &mut self,
        output_queue: Option<&OutputQueueDataV2>,
        input_queue: Option<&InputQueueDataV2>,
        _on_chain_root: [u8; 32],
    ) -> Result<()> {
        if let Some(oq) = output_queue {
            debug!(
                "Merging {} deduplicated nodes from output queue",
                oq.nodes.len()
            );

            for (node_idx, node_hash) in oq.nodes.iter().zip(oq.node_hashes.iter()) {
                let (level, position) = decode_node_index(*node_idx);
                Self::set_node_in_tree(
                    &mut self.tree,
                    level as usize,
                    position as usize,
                    *node_hash,
                )?;
            }

            for (leaf_idx, leaf_hash) in oq.leaf_indices.iter().zip(oq.old_leaves.iter()) {
                Self::set_node_in_tree(&mut self.tree, 0, *leaf_idx as usize, *leaf_hash)?;
            }
        }

        if let Some(iq) = input_queue {
            debug!(
                "Merging {} deduplicated nodes from input queue",
                iq.nodes.len()
            );

            for (node_idx, node_hash) in iq.nodes.iter().zip(iq.node_hashes.iter()) {
                let (level, position) = decode_node_index(*node_idx);
                Self::set_node_in_tree(
                    &mut self.tree,
                    level as usize,
                    position as usize,
                    *node_hash,
                )?;
            }

            for (leaf_idx, leaf_hash) in iq.leaf_indices.iter().zip(iq.current_leaves.iter()) {
                Self::set_node_in_tree(&mut self.tree, 0, *leaf_idx as usize, *leaf_hash)?;
            }
        }

        Ok(())
    }
}

impl TreeState {
    pub fn from_v2_response(
        output_queue: Option<&OutputQueueDataV2>,
        input_queue: Option<&InputQueueDataV2>,
    ) -> Result<Self> {
        let mut tree = MerkleTree::<Poseidon>::new(TREE_HEIGHT, 0);
        let mut cached_root = [0u8; 32];

        if let Some(oq) = output_queue {
            debug!(
                "Loading {} deduplicated nodes from output queue",
                oq.nodes.len()
            );

            for (node_idx, node_hash) in oq.nodes.iter().zip(oq.node_hashes.iter()) {
                let (level, position) = decode_node_index(*node_idx);
                Self::set_node_in_tree(&mut tree, level as usize, position as usize, *node_hash)?;
            }

            for (leaf_idx, leaf_hash) in oq.leaf_indices.iter().zip(oq.old_leaves.iter()) {
                Self::set_node_in_tree(&mut tree, 0, *leaf_idx as usize, *leaf_hash)?;
            }

            cached_root = oq.initial_root;
        }

        if let Some(iq) = input_queue {
            debug!(
                "Loading {} deduplicated nodes from input queue",
                iq.nodes.len()
            );

            for (node_idx, node_hash) in iq.nodes.iter().zip(iq.node_hashes.iter()) {
                let (level, position) = decode_node_index(*node_idx);
                Self::set_node_in_tree(&mut tree, level as usize, position as usize, *node_hash)?;
            }

            for (leaf_idx, leaf_hash) in iq.leaf_indices.iter().zip(iq.current_leaves.iter()) {
                Self::set_node_in_tree(&mut tree, 0, *leaf_idx as usize, *leaf_hash)?;
            }

            if output_queue.is_none() {
                cached_root = iq.initial_root;
            }
        }

        // NOTE: We do NOT verify the tree root here because the tree is loaded with
        // deduplicated nodes from the indexer. The tree structure is incomplete but sufficient
        // for incremental proof generation via the staging tree mechanism.
        debug!(
            "Tree loaded from indexer: cached_root={:?}, nodes={} total (deduplicated)",
            &cached_root[..8],
            tree.layers.iter().map(|l| l.len()).sum::<usize>()
        );

        Ok(Self {
            tree,
            cached_root,
            root_dirty: false,
        })
    }

    fn set_node_in_tree(
        tree: &mut MerkleTree<Poseidon>,
        level: usize,
        position: usize,
        hash: [u8; 32],
    ) -> Result<()> {
        if tree.layers[level].len() <= position {
            tree.layers[level].resize(position + 1, [0u8; 32]);
        }
        tree.layers[level][position] = hash;
        Ok(())
    }

    pub fn get_proof(&self, leaf_index: u64) -> Result<Vec<[u8; 32]>> {
        trace!("Building proof for leaf {}", leaf_index);

        let proof = self
            .tree
            .get_proof_of_leaf(leaf_index as usize, true)
            .map_err(|e| anyhow!("Failed to get proof for leaf {}: {:?}", leaf_index, e))?;

        trace!(
            "Built proof for leaf {} with {} siblings",
            leaf_index,
            proof.len()
        );
        Ok(proof)
    }

    pub fn update_leaf(&mut self, leaf_index: u64, new_leaf: [u8; 32]) -> Result<()> {
        self.tree
            .update(&new_leaf, leaf_index as usize)
            .map_err(|e| anyhow!("Failed to update leaf {}: {:?}", leaf_index, e))?;

        self.root_dirty = true;
        Ok(())
    }

    pub fn current_root(&mut self) -> [u8; 32] {
        if self.root_dirty {
            self.cached_root = self.tree.root();
            self.root_dirty = false;
        }
        self.cached_root
    }

    pub fn get_cached_root(&self) -> [u8; 32] {
        self.cached_root
    }

    pub fn node_count(&self) -> usize {
        self.tree.layers.iter().map(|layer| layer.len()).sum()
    }

    pub fn get_leaf(&self, leaf_index: u64) -> Option<[u8; 32]> {
        self.tree.layers[0].get(leaf_index as usize).copied()
    }

    pub fn capacity(&self) -> usize {
        self.tree.layers[0].capacity()
    }

    pub fn shrink_to_fit(&mut self) {
        for layer in self.tree.layers.iter_mut() {
            layer.shrink_to_fit();
        }
    }

    pub fn from_root_and_capacity(root: [u8; 32], capacity: usize) -> Self {
        let mut tree = MerkleTree::<Poseidon>::new(TREE_HEIGHT, 0);
        if capacity > 0 {
            tree.layers[0].reserve(capacity);
        }
        Self {
            tree,
            cached_root: root,
            root_dirty: false,
        }
    }

    pub fn batch_update_leaves(&mut self, updates: &[(u64, [u8; 32])]) -> Result<()> {
        if updates.is_empty() {
            return Ok(());
        }

        let convert_start = std::time::Instant::now();
        let tree_updates: Vec<(usize, [u8; 32])> = updates
            .iter()
            .map(|(idx, leaf)| (*idx as usize, *leaf))
            .collect();
        let convert_time = convert_start.elapsed();

        let update_start = std::time::Instant::now();
        self.tree
            .batch_update(&tree_updates)
            .map_err(|e| anyhow!("Failed to batch update leaves: {:?}", e))?;
        let update_time = update_start.elapsed();

        let root_start = std::time::Instant::now();
        self.cached_root = self.tree.root();
        self.root_dirty = false;
        let root_time = root_start.elapsed();

        trace!(
            "batch_update_leaves: {} updates | convert={:?} update={:?} root={:?}",
            updates.len(),
            convert_time,
            update_time,
            root_time
        );

        Ok(())
    }

    pub fn clear(&mut self) {
        self.tree = MerkleTree::<Poseidon>::new(TREE_HEIGHT, 0);
        self.cached_root = [0u8; 32];
        self.root_dirty = false;
    }

    pub fn create_staging(&self) -> StagingTree {
        StagingTree {
            tree: self.tree.clone(),
            base_root: self.cached_root,
            updates: Vec::new(),
        }
    }

    pub fn set_cached_root(&mut self, root: [u8; 32]) {
        self.cached_root = root;
        self.root_dirty = false;
    }
}

#[inline]
fn decode_node_index(encoded: u64) -> (u8, u64) {
    let level = (encoded >> 56) as u8;
    let position = encoded & 0x00FFFFFFFFFFFFFF;
    (level, position)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree_state() {
        let mut state = TreeState::from_v2_response(None, None).unwrap();
        assert_eq!(state.current_root(), [0u8; 32]);
    }

    #[test]
    fn test_decode_node_index() {
        let encoded = 0u64;
        assert_eq!(decode_node_index(encoded), (0, 0));

        let encoded = ((15u64) << 56) | 12345;
        assert_eq!(decode_node_index(encoded), (15, 12345));

        let encoded = ((31u64) << 56) | 0;
        assert_eq!(decode_node_index(encoded), (31, 0));

        let encoded = 0x00FFFFFFFFFFFFFF;
        assert_eq!(decode_node_index(encoded), (0, 0x00FFFFFFFFFFFFFF));
    }
}
