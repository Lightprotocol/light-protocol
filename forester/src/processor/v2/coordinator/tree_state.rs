use anyhow::{anyhow, Result};
use light_client::indexer::{InputQueueDataV2, OutputQueueDataV2};
use light_sparse_merkle_tree::changelog::ChangelogEntry;
use std::collections::HashMap;
use tracing::{debug, trace};

pub const TREE_HEIGHT: u8 = 32;

/// Encode tree node position as a single u64.
///
/// Format: [level: u8][position: 56 bits]
/// - Level 0 = leaves
/// - Level 31 = root
#[inline]
pub fn encode_node_index(level: u8, position: u64) -> u64 {
    debug_assert!(
        level < TREE_HEIGHT,
        "Level {} exceeds tree height {}",
        level,
        TREE_HEIGHT
    );
    ((level as u64) << 56) | position
}

/// Decode tree node index back to (level, position).
#[inline]
pub fn decode_node_index(encoded: u64) -> (u8, u64) {
    let level = (encoded >> 56) as u8;
    let position = encoded & 0x00FFFFFFFFFFFFFF;
    (level, position)
}

#[derive(Debug, Clone)]
pub struct TreeState {
    pub nodes: HashMap<u64, [u8; 32]>,
    current_root: [u8; 32],
}

impl TreeState {
    /// Create TreeState from V2 queue response data.
    ///
    /// Loads deduplicated nodes from both output and input queues.
    /// This approach significantly reduces memory usage and network overhead.
    pub fn from_v2_response(
        output_queue: Option<&OutputQueueDataV2>,
        input_queue: Option<&InputQueueDataV2>,
    ) -> Result<Self> {
        let mut nodes = HashMap::new();
        let mut initial_root = [0u8; 32];

        // Load nodes from output queue (append operations)
        if let Some(oq) = output_queue {
            debug!(
                "Loading {} deduplicated nodes from output queue",
                oq.nodes.len()
            );

            for (node_idx, node_hash) in oq.nodes.iter().zip(oq.node_hashes.iter()) {
                nodes.insert(*node_idx, *node_hash);
            }

            for (leaf_idx, leaf_hash) in oq.leaf_indices.iter().zip(oq.old_leaves.iter()) {
                let node_idx = encode_node_index(0, *leaf_idx);
                nodes.insert(node_idx, *leaf_hash);
            }

            initial_root = oq.initial_root;
        }

        // Load nodes from input queue (nullify operations)
        if let Some(iq) = input_queue {
            debug!(
                "Loading {} deduplicated nodes from input queue",
                iq.nodes.len()
            );

            for (node_idx, node_hash) in iq.nodes.iter().zip(iq.node_hashes.iter()) {
                nodes.insert(*node_idx, *node_hash);
            }

            for (leaf_idx, leaf_hash) in iq.leaf_indices.iter().zip(iq.current_leaves.iter()) {
                let node_idx = encode_node_index(0, *leaf_idx);
                nodes.insert(node_idx, *leaf_hash);
            }

            // Use input queue root if output queue wasn't provided
            if output_queue.is_none() {
                initial_root = iq.initial_root;
            }
        }

        debug!("TreeState initialized with {} nodes", nodes.len());

        Ok(Self {
            nodes,
            current_root: initial_root,
        })
    }

    /// Generate Merkle proof for a given leaf index.
    ///
    /// Returns sibling hashes from leaf to root (height = TREE_HEIGHT).
    pub fn get_proof(&self, leaf_index: u64) -> Result<Vec<[u8; 32]>> {
        let mut proof = Vec::with_capacity(TREE_HEIGHT as usize);
        let mut pos = leaf_index;

        trace!("Building proof for leaf {}", leaf_index);

        for level in 0..TREE_HEIGHT {
            let sibling_pos = if pos % 2 == 0 { pos + 1 } else { pos - 1 };
            let node_idx = encode_node_index(level, sibling_pos);

            let sibling_hash = self.nodes.get(&node_idx).ok_or_else(|| {
                anyhow!(
                    "Missing node at level {} position {} (encoded: 0x{:016x}) for leaf {}",
                    level,
                    sibling_pos,
                    node_idx,
                    leaf_index
                )
            })?;

            proof.push(*sibling_hash);
            pos /= 2;
        }

        trace!(
            "Built proof for leaf {} with {} siblings",
            leaf_index,
            proof.len()
        );
        Ok(proof)
    }

    /// Apply changelog entry to update tree nodes.
    ///
    /// This updates:
    /// 1. Sibling nodes along the path to root
    /// 2. The leaf value
    /// 3. The current root
    pub fn apply_changelog(
        &mut self,
        changelog: &ChangelogEntry<32>,
        new_leaf: [u8; 32],
        new_root: [u8; 32],
    ) {
        let mut pos = changelog.index;

        // Update sibling nodes along the path
        for (level, &sibling_hash_opt) in changelog.path.iter().enumerate() {
            if let Some(sibling_hash) = sibling_hash_opt {
                let sibling_pos = if pos % 2 == 0 { pos + 1 } else { pos - 1 };
                let node_idx = encode_node_index(level as u8, sibling_pos);
                self.nodes.insert(node_idx, sibling_hash);
            }
            pos /= 2;
        }

        let leaf_idx = encode_node_index(0, changelog.index);
        self.nodes.insert(leaf_idx, new_leaf);
        self.current_root = new_root;
    }

    /// Get the current root hash.
    pub fn current_root(&self) -> [u8; 32] {
        self.current_root
    }

    /// Get the total number of nodes stored.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_encoding() {
        // Test encoding/decoding round-trip
        let level = 15u8;
        let position = 12345u64;
        let encoded = encode_node_index(level, position);
        let (decoded_level, decoded_position) = decode_node_index(encoded);
        assert_eq!(level, decoded_level);
        assert_eq!(position, decoded_position);
    }

    #[test]
    fn test_node_encoding_boundaries() {
        // Test level 0
        let encoded = encode_node_index(0, 0);
        assert_eq!(decode_node_index(encoded), (0, 0));

        // Test max level
        let encoded = encode_node_index(31, 0);
        assert_eq!(decode_node_index(encoded), (31, 0));

        // Test large position
        let encoded = encode_node_index(0, 0x00FFFFFFFFFFFFFF);
        assert_eq!(decode_node_index(encoded), (0, 0x00FFFFFFFFFFFFFF));
    }

    #[test]
    fn test_empty_tree_state() {
        let state = TreeState::from_v2_response(None, None).unwrap();
        assert_eq!(state.node_count(), 0);
        assert_eq!(state.current_root(), [0u8; 32]);
    }
}
