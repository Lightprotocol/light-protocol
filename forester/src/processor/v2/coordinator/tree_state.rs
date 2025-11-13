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
