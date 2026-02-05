use super::super::IndexerError;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OutputQueueData {
    pub leaf_indices: Vec<u64>,
    pub account_hashes: Vec<[u8; 32]>,
    pub old_leaves: Vec<[u8; 32]>,
    pub first_queue_index: u64,
    /// The tree's next_index - where new leaves will be appended
    pub next_index: u64,
    /// Pre-computed hash chains per ZKP batch (from on-chain)
    pub leaves_hash_chains: Vec<[u8; 32]>,
}

/// V2 Input Queue Data
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InputQueueData {
    pub leaf_indices: Vec<u64>,
    pub account_hashes: Vec<[u8; 32]>,
    pub current_leaves: Vec<[u8; 32]>,
    pub tx_hashes: Vec<[u8; 32]>,
    /// Pre-computed nullifiers from indexer
    pub nullifiers: Vec<[u8; 32]>,
    pub first_queue_index: u64,
    /// Pre-computed hash chains per ZKP batch (from on-chain)
    pub leaves_hash_chains: Vec<[u8; 32]>,
}

/// State queue data with shared tree nodes for output and input queues
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StateQueueData {
    /// Shared deduplicated tree nodes for state queues (output + input)
    /// node_index encoding: (level << 56) | position
    pub nodes: Vec<u64>,
    pub node_hashes: Vec<[u8; 32]>,
    /// Initial root for the state tree (shared by output and input queues)
    pub initial_root: [u8; 32],
    /// Sequence number of the root
    pub root_seq: u64,
    /// Output queue data (if requested)
    pub output_queue: Option<OutputQueueData>,
    /// Input queue data (if requested)
    pub input_queue: Option<InputQueueData>,
}

/// V2 Address Queue Data with deduplicated nodes
/// Proofs are reconstructed from `nodes`/`node_hashes` using `low_element_indices`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AddressQueueData {
    pub addresses: Vec<[u8; 32]>,
    pub low_element_values: Vec<[u8; 32]>,
    pub low_element_next_values: Vec<[u8; 32]>,
    pub low_element_indices: Vec<u64>,
    pub low_element_next_indices: Vec<u64>,
    /// Deduplicated node indices - encoding: (level << 56) | position
    pub nodes: Vec<u64>,
    /// Hashes corresponding to each node index
    pub node_hashes: Vec<[u8; 32]>,
    pub initial_root: [u8; 32],
    pub leaves_hash_chains: Vec<[u8; 32]>,
    pub subtrees: Vec<[u8; 32]>,
    pub start_index: u64,
    pub root_seq: u64,
}

impl AddressQueueData {
    /// Reconstruct a merkle proof for a given low_element_index from the deduplicated nodes.
    /// The tree_height is needed to know how many levels to traverse.
    pub fn reconstruct_proof(
        &self,
        address_idx: usize,
        tree_height: u8,
    ) -> Result<Vec<[u8; 32]>, IndexerError> {
        let leaf_index = self.low_element_indices[address_idx];
        let mut proof = Vec::with_capacity(tree_height as usize);
        let mut pos = leaf_index;

        for level in 0..tree_height {
            let sibling_pos = if pos.is_multiple_of(2) {
                pos + 1
            } else {
                pos - 1
            };
            let sibling_idx = Self::encode_node_index(level, sibling_pos);

            if let Some(hash_idx) = self.nodes.iter().position(|&n| n == sibling_idx) {
                proof.push(self.node_hashes[hash_idx]);
            } else {
                return Err(IndexerError::MissingResult {
                    context: "reconstruct_proof".to_string(),
                    message: format!(
                        "Missing proof node at level {} position {} (encoded: {})",
                        level, sibling_pos, sibling_idx
                    ),
                });
            }
            pos /= 2;
        }

        Ok(proof)
    }

    /// Reconstruct all proofs for all addresses
    pub fn reconstruct_all_proofs(
        &self,
        tree_height: u8,
    ) -> Result<Vec<Vec<[u8; 32]>>, IndexerError> {
        (0..self.addresses.len())
            .map(|i| self.reconstruct_proof(i, tree_height))
            .collect()
    }

    /// Encode node index: (level << 56) | position
    #[inline]
    fn encode_node_index(level: u8, position: u64) -> u64 {
        ((level as u64) << 56) | position
    }
}

/// V2 Queue Elements Result with deduplicated node data
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QueueElementsResult {
    pub state_queue: Option<StateQueueData>,
    pub address_queue: Option<AddressQueueData>,
}
