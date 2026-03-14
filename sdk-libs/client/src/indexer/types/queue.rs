use std::collections::HashMap;

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
    /// Reconstruct a single merkle proof for a given address index.
    #[cfg(test)]
    fn reconstruct_proof<const HEIGHT: usize>(
        &self,
        address_idx: usize,
    ) -> Result<[[u8; 32]; HEIGHT], IndexerError> {
        let node_lookup = self.build_node_lookup();
        self.reconstruct_proof_with_lookup::<HEIGHT>(address_idx, &node_lookup)
    }

    /// Reconstruct a contiguous batch of proofs while reusing a single node lookup table.
    pub fn reconstruct_proofs<const HEIGHT: usize>(
        &self,
        address_range: std::ops::Range<usize>,
    ) -> Result<Vec<[[u8; 32]; HEIGHT]>, IndexerError> {
        self.validate_proof_height::<HEIGHT>()?;
        let node_lookup = self.build_node_lookup();
        let mut proofs = Vec::with_capacity(address_range.len());

        for address_idx in address_range {
            proofs.push(self.reconstruct_proof_with_lookup::<HEIGHT>(address_idx, &node_lookup)?);
        }

        Ok(proofs)
    }

    /// Reconstruct all proofs for all addresses
    pub fn reconstruct_all_proofs<const HEIGHT: usize>(
        &self,
    ) -> Result<Vec<[[u8; 32]; HEIGHT]>, IndexerError> {
        self.validate_proof_height::<HEIGHT>()?;
        self.reconstruct_proofs::<HEIGHT>(0..self.addresses.len())
    }

    fn build_node_lookup(&self) -> HashMap<u64, usize> {
        let mut lookup = HashMap::with_capacity(self.nodes.len());
        for (idx, node) in self.nodes.iter().copied().enumerate() {
            lookup.entry(node).or_insert(idx);
        }
        lookup
    }

    fn reconstruct_proof_with_lookup<const HEIGHT: usize>(
        &self,
        address_idx: usize,
        node_lookup: &HashMap<u64, usize>,
    ) -> Result<[[u8; 32]; HEIGHT], IndexerError> {
        self.validate_proof_height::<HEIGHT>()?;
        let leaf_index = *self.low_element_indices.get(address_idx).ok_or_else(|| {
            IndexerError::MissingResult {
                context: "reconstruct_proof".to_string(),
                message: format!(
                    "address_idx {} out of bounds for low_element_indices (len {})",
                    address_idx,
                    self.low_element_indices.len(),
                ),
            }
        })?;
        let mut proof = [[0u8; 32]; HEIGHT];
        let mut pos = leaf_index;

        for (level, proof_element) in proof.iter_mut().enumerate() {
            let sibling_pos = if pos.is_multiple_of(2) {
                pos + 1
            } else {
                pos - 1
            };
            let sibling_idx = Self::encode_node_index(level, sibling_pos);
            let hash_idx = node_lookup.get(&sibling_idx).copied().ok_or_else(|| {
                IndexerError::MissingResult {
                    context: "reconstruct_proof".to_string(),
                    message: format!(
                        "Missing proof node at level {} position {} (encoded: {})",
                        level, sibling_pos, sibling_idx
                    ),
                }
            })?;
            let hash =
                self.node_hashes
                    .get(hash_idx)
                    .ok_or_else(|| IndexerError::MissingResult {
                        context: "reconstruct_proof".to_string(),
                        message: format!(
                            "node_hashes index {} out of bounds (len {})",
                            hash_idx,
                            self.node_hashes.len(),
                        ),
                    })?;
            *proof_element = *hash;
            pos /= 2;
        }

        Ok(proof)
    }

    /// Encode node index: (level << 56) | position
    #[inline]
    fn encode_node_index(level: usize, position: u64) -> u64 {
        ((level as u64) << 56) | position
    }

    fn validate_proof_height<const HEIGHT: usize>(&self) -> Result<(), IndexerError> {
        if HEIGHT == Self::ADDRESS_TREE_HEIGHT {
            return Ok(());
        }

        Err(IndexerError::InvalidParameters(format!(
            "address queue proofs require HEIGHT={} but got HEIGHT={}",
            Self::ADDRESS_TREE_HEIGHT,
            HEIGHT
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, hint::black_box, time::Instant};

    use super::AddressQueueData;

    fn hash_from_node(node_index: u64) -> [u8; 32] {
        let mut hash = [0u8; 32];
        hash[..8].copy_from_slice(&node_index.to_le_bytes());
        hash[8..16].copy_from_slice(&node_index.rotate_left(17).to_le_bytes());
        hash[16..24].copy_from_slice(&node_index.rotate_right(9).to_le_bytes());
        hash[24..32].copy_from_slice(&(node_index ^ 0xA5A5_A5A5_A5A5_A5A5).to_le_bytes());
        hash
    }

    fn build_queue_data<const HEIGHT: usize>(num_addresses: usize) -> AddressQueueData {
        let low_element_indices = (0..num_addresses)
            .map(|i| (i as u64).saturating_mul(2))
            .collect::<Vec<_>>();
        let mut nodes = BTreeMap::new();

        for &leaf_index in &low_element_indices {
            let mut pos = leaf_index;
            for level in 0..HEIGHT {
                let sibling_pos = if pos.is_multiple_of(2) {
                    pos + 1
                } else {
                    pos - 1
                };
                let node_index = ((level as u64) << 56) | sibling_pos;
                nodes
                    .entry(node_index)
                    .or_insert_with(|| hash_from_node(node_index));
                pos /= 2;
            }
        }

        let (nodes, node_hashes): (Vec<_>, Vec<_>) = nodes.into_iter().unzip();

        AddressQueueData {
            addresses: vec![[0u8; 32]; num_addresses],
            low_element_values: vec![[1u8; 32]; num_addresses],
            low_element_next_values: vec![[2u8; 32]; num_addresses],
            low_element_indices,
            low_element_next_indices: (0..num_addresses).map(|i| (i as u64) + 1).collect(),
            nodes,
            node_hashes,
            initial_root: [9u8; 32],
            leaves_hash_chains: vec![[3u8; 32]; num_addresses.max(1)],
            subtrees: vec![[4u8; 32]; HEIGHT],
            start_index: 0,
            root_seq: 0,
        }
    }

    #[test]
    fn batched_reconstruction_matches_individual_reconstruction() {
        let queue = build_queue_data::<40>(128);

        let expected = (0..queue.addresses.len())
            .map(|i| queue.reconstruct_proof::<40>(i).unwrap())
            .collect::<Vec<_>>();
        let actual = queue
            .reconstruct_proofs::<40>(0..queue.addresses.len())
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    #[ignore = "profiling helper"]
    fn profile_reconstruct_proofs_batch() {
        const HEIGHT: usize = 40;
        const NUM_ADDRESSES: usize = 2_048;
        const ITERS: usize = 25;

        let queue = build_queue_data::<HEIGHT>(NUM_ADDRESSES);

        let baseline_start = Instant::now();
        for _ in 0..ITERS {
            let proofs = (0..queue.addresses.len())
                .map(|i| queue.reconstruct_proof::<HEIGHT>(i).unwrap())
                .collect::<Vec<_>>();
            black_box(proofs);
        }
        let baseline = baseline_start.elapsed();

        let batched_start = Instant::now();
        for _ in 0..ITERS {
            black_box(
                queue
                    .reconstruct_proofs::<HEIGHT>(0..queue.addresses.len())
                    .unwrap(),
            );
        }
        let batched = batched_start.elapsed();

        println!(
            "queue reconstruction profile: addresses={}, height={}, iters={}, individual={:?}, batched={:?}, speedup={:.2}x",
            NUM_ADDRESSES,
            HEIGHT,
            ITERS,
            baseline,
            batched,
            baseline.as_secs_f64() / batched.as_secs_f64(),
        );
    }
}

/// V2 Queue Elements Result with deduplicated node data
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QueueElementsResult {
    pub state_queue: Option<StateQueueData>,
    pub address_queue: Option<AddressQueueData>,
}
