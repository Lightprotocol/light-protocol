use anyhow::{anyhow, Result};
use light_client::indexer::{InputQueueDataV2, OutputQueueDataV2};
use light_hasher::Poseidon;
use light_merkle_tree_reference::MerkleTree;
use tracing::{debug, trace};

pub const TREE_HEIGHT: usize = 32;

/// Wrapper around reference MerkleTree for forester use.
///
/// Provides a simplified interface for loading deduplicated nodes from indexer
/// and generating proofs without needing changelog adjustments.
#[derive(Debug, Clone)]
pub struct TreeState {
    tree: MerkleTree<Poseidon>,
    /// Track root separately since we load sparse nodes from indexer
    /// and can't rely on tree's computed root
    cached_root: [u8; 32],
}

impl TreeState {
    /// Create TreeState from V2 queue response data.
    ///
    /// Loads deduplicated nodes from both output and input queues into a reference MerkleTree.
    /// The reference tree uses layered storage which provides better cache locality.
    pub fn from_v2_response(
        output_queue: Option<&OutputQueueDataV2>,
        input_queue: Option<&InputQueueDataV2>,
    ) -> Result<Self> {
        // Create new merkle tree with canopy depth 0 (we store all nodes)
        let mut tree = MerkleTree::<Poseidon>::new(TREE_HEIGHT, 0);

        let mut cached_root = [0u8; 32];
        let mut node_count = 0;

        // Load nodes from output queue (append operations)
        if let Some(oq) = output_queue {
            debug!(
                "Loading {} deduplicated nodes from output queue",
                oq.nodes.len()
            );

            // Populate tree layers from indexer nodes
            for (node_idx, node_hash) in oq.nodes.iter().zip(oq.node_hashes.iter()) {
                let (level, position) = decode_node_index(*node_idx);
                Self::set_node_in_tree(&mut tree, level as usize, position as usize, *node_hash)?;
                node_count += 1;
            }

            // Add leaves
            for (leaf_idx, leaf_hash) in oq.leaf_indices.iter().zip(oq.old_leaves.iter()) {
                Self::set_node_in_tree(&mut tree, 0, *leaf_idx as usize, *leaf_hash)?;
                node_count += 1;
            }

            cached_root = oq.initial_root;
        }

        // Load nodes from input queue (nullify operations)
        if let Some(iq) = input_queue {
            debug!(
                "Loading {} deduplicated nodes from input queue",
                iq.nodes.len()
            );

            for (node_idx, node_hash) in iq.nodes.iter().zip(iq.node_hashes.iter()) {
                let (level, position) = decode_node_index(*node_idx);
                Self::set_node_in_tree(&mut tree, level as usize, position as usize, *node_hash)?;
                node_count += 1;
            }

            for (leaf_idx, leaf_hash) in iq.leaf_indices.iter().zip(iq.current_leaves.iter()) {
                Self::set_node_in_tree(&mut tree, 0, *leaf_idx as usize, *leaf_hash)?;
                node_count += 1;
            }

            // Use input queue root if output queue wasn't provided
            if output_queue.is_none() {
                cached_root = iq.initial_root;
            }
        }

        debug!("TreeState initialized with {} nodes, root: {:?}", node_count, &cached_root[..8]);

        Ok(Self { tree, cached_root })
    }

    /// Helper to set a node in the tree's layer structure.
    fn set_node_in_tree(
        tree: &mut MerkleTree<Poseidon>,
        level: usize,
        position: usize,
        hash: [u8; 32],
    ) -> Result<()> {
        // Ensure the layer has enough capacity
        if tree.layers[level].len() <= position {
            tree.layers[level].resize(position + 1, [0u8; 32]);
        }
        tree.layers[level][position] = hash;
        Ok(())
    }

    /// Generate Merkle proof for a given leaf index.
    ///
    /// Returns sibling hashes from leaf to root (height = TREE_HEIGHT).
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

    /// Update a leaf and propagate changes up the tree.
    ///
    /// This replaces the old `apply_changelog` approach - we now update the tree
    /// immediately instead of accumulating changelogs for later adjustment.
    pub fn update_leaf(&mut self, leaf_index: u64, new_leaf: [u8; 32]) -> Result<()> {
        self.tree
            .update(&new_leaf, leaf_index as usize)
            .map_err(|e| anyhow!("Failed to update leaf {}: {:?}", leaf_index, e))?;

        // Update cached root
        self.cached_root = self.tree.root();
        Ok(())
    }

    /// Get the current root hash.
    pub fn current_root(&self) -> [u8; 32] {
        self.cached_root
    }

    /// Get the total number of nodes stored across all layers.
    pub fn node_count(&self) -> usize {
        self.tree.layers.iter().map(|layer| layer.len()).sum()
    }

    /// Get a leaf value by index.
    pub fn get_leaf(&self, leaf_index: u64) -> Option<[u8; 32]> {
        self.tree.layers[0].get(leaf_index as usize).copied()
    }

    /// Shrink tree storage to fit current data.
    ///
    /// This reduces memory usage by shrinking internal vectors to their minimum size.
    /// Call this after batches are confirmed on-chain and no longer need the intermediate state.
    pub fn shrink_to_fit(&mut self) {
        for layer in self.tree.layers.iter_mut() {
            layer.shrink_to_fit();
        }
    }

    /// Clear all tree data and reset to empty state.
    ///
    /// Use this when reloading tree state from indexer or switching epochs.
    pub fn clear(&mut self) {
        self.tree = MerkleTree::<Poseidon>::new(TREE_HEIGHT, 0);
        self.cached_root = [0u8; 32];
    }
}

/// Decode tree node index back to (level, position).
///
/// Format: [level: u8][position: 56 bits]
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
        let state = TreeState::from_v2_response(None, None).unwrap();
        // Empty tree should have zero root
        assert_eq!(state.current_root(), [0u8; 32]);
    }

    #[test]
    fn test_decode_node_index() {
        // Test level 0, position 0
        let encoded = 0u64;
        assert_eq!(decode_node_index(encoded), (0, 0));

        // Test level 15, position 12345
        let encoded = ((15u64) << 56) | 12345;
        assert_eq!(decode_node_index(encoded), (15, 12345));

        // Test max level (31)
        let encoded = ((31u64) << 56) | 0;
        assert_eq!(decode_node_index(encoded), (31, 0));

        // Test large position
        let encoded = 0x00FFFFFFFFFFFFFF;
        assert_eq!(decode_node_index(encoded), (0, 0x00FFFFFFFFFFFFFF));
    }
}
