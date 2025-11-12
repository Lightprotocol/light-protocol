/// Type definitions for the state tree coordinator.
use light_client::indexer::MerkleProofWithContext;

/// Type of batch operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchType {
    /// Append operation (output queue).
    Append,
    /// Nullify operation (input queue).
    Nullify,
}

impl BatchType {
    /// Returns a human-readable name for logging.
    pub fn as_str(&self) -> &'static str {
        match self {
            BatchType::Append => "append",
            BatchType::Nullify => "nullify",
        }
    }
}

/// Prepared batch data ready for proof generation.
#[derive(Debug)]
pub enum PreparedBatch {
    Append(light_prover_client::proof_types::batch_append::BatchAppendsCircuitInputs),
    Nullify(light_prover_client::proof_types::batch_update::BatchUpdateCircuitInputs),
}

impl PreparedBatch {
    /// Get the batch type.
    pub fn batch_type(&self) -> BatchType {
        match self {
            PreparedBatch::Append(_) => BatchType::Append,
            PreparedBatch::Nullify(_) => BatchType::Nullify,
        }
    }
}

/// Data for append batches from the output queue.
#[derive(Debug)]
pub struct AppendQueueData {
    pub queue_elements: Vec<MerkleProofWithContext>,
    pub leaves_hash_chains: Vec<[u8; 32]>,
    pub zkp_batch_size: u16,
}

impl AppendQueueData {
    /// Total number of elements across all batches.
    pub fn total_elements(&self) -> usize {
        self.queue_elements.len()
    }

    /// Number of batches.
    pub fn num_batches(&self) -> usize {
        self.leaves_hash_chains.len()
    }
}

/// Data for nullify batches from the input queue.
#[derive(Debug)]
pub struct NullifyQueueData {
    pub queue_elements: Vec<MerkleProofWithContext>,
    pub leaves_hash_chains: Vec<[u8; 32]>,
    pub zkp_batch_size: u16,
    pub num_inserted_zkps: u64,
}

impl NullifyQueueData {
    /// Total number of elements across all batches.
    pub fn total_elements(&self) -> usize {
        self.queue_elements.len()
    }

    /// Number of batches.
    pub fn num_batches(&self) -> usize {
        self.leaves_hash_chains.len()
    }
}

/// State accumulated during batch preparation phase.
///
/// Note: We no longer accumulate changelogs - the tree is updated immediately
/// after each operation, eliminating the need for proof adjustments.
pub struct PreparationState {
    pub tree_state: super::tree_state::TreeState,
    pub current_root: [u8; 32],
    pub append_batch_index: usize,
    pub nullify_batch_index: usize,
    pub append_leaf_indices: Vec<u64>,
}

impl PreparationState {
    /// Create new preparation state from initial tree state.
    pub fn new(tree_state: super::tree_state::TreeState, append_leaf_indices: Vec<u64>) -> Self {
        let initial_root = tree_state.current_root();
        Self {
            tree_state,
            current_root: initial_root,
            append_batch_index: 0,
            nullify_batch_index: 0,
            append_leaf_indices,
        }
    }
}
