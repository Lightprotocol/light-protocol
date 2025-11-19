/// Type definitions for the state tree coordinator.
use light_client::indexer::MerkleProofWithContext;

/// Type of batch operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchType {
    /// Append operation (output queue).
    Append,
    /// Nullify operation (input queue).
    Nullify,
    /// Address append operation (address queue).
    Address,
}

impl BatchType {
    /// Returns a human-readable name for logging.
    pub fn as_str(&self) -> &'static str {
        match self {
            BatchType::Append => "append",
            BatchType::Nullify => "nullify",
            BatchType::Address => "address",
        }
    }
}

/// Prepared batch data ready for proof generation.
#[derive(Debug, Clone)]
pub enum PreparedBatch {
    Append(light_prover_client::proof_types::batch_append::BatchAppendsCircuitInputs),
    Nullify(light_prover_client::proof_types::batch_update::BatchUpdateCircuitInputs),
    Address(light_prover_client::proof_types::batch_address_append::BatchAddressAppendInputs),
}

impl PreparedBatch {
    /// Get the batch type.
    pub fn batch_type(&self) -> BatchType {
        match self {
            PreparedBatch::Append(_) => BatchType::Append,
            PreparedBatch::Nullify(_) => BatchType::Nullify,
            PreparedBatch::Address(_) => BatchType::Address,
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

/// Data for address append batches from the address queue.
#[derive(Debug)]
pub struct AddressQueueData {
    /// Addresses to be inserted.
    pub addresses: Vec<[u8; 32]>,
    /// Low element values for non-inclusion proofs.
    pub low_element_values: Vec<[u8; 32]>,
    /// Next values for low elements.
    pub low_element_next_values: Vec<[u8; 32]>,
    /// Indices of low elements in the tree.
    pub low_element_indices: Vec<u64>,
    /// Next indices for low elements.
    pub low_element_next_indices: Vec<u64>,
    /// Merkle proofs for low elements.
    pub low_element_proofs: Vec<Vec<[u8; 32]>>,
    /// Hash chains for batches.
    pub leaves_hash_chains: Vec<[u8; 32]>,
    /// ZKP batch size.
    pub zkp_batch_size: u16,
    /// Subtrees for the sparse merkle tree.
    pub subtrees: Vec<[u8; 32]>,
    /// Start index for the first batch.
    pub start_index: u64,
}

impl AddressQueueData {
    /// Total number of elements across all batches.
    pub fn total_elements(&self) -> usize {
        self.addresses.len()
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
    pub append_batch_index: usize,
    pub nullify_batch_index: usize,
    pub append_leaf_indices: Vec<u64>,
    /// Persistent staging tree that accumulates ALL updates across all batches.
    /// This ensures that each batch sees updates from previous batches in the same cycle.
    pub staging: super::tree_state::StagingTree,
}

impl PreparationState {
    /// Create new preparation state from staging tree.
    pub fn new(staging: super::tree_state::StagingTree, append_leaf_indices: Vec<u64>) -> Self {
        Self {
            append_batch_index: 0,
            nullify_batch_index: 0,
            append_leaf_indices,
            staging,
        }
    }

    /// Create new preparation state with a cached staging tree.
    /// Reuses the cached staging tree as-is with all its accumulated updates.
    /// The batch indices are reset to 0 because the new queue data contains only new elements,
    /// but the staging tree already contains all previous updates, ensuring correct proofs.
    pub fn with_cached_staging(
        append_leaf_indices: Vec<u64>,
        mut staging: super::tree_state::StagingTree,
        output_queue: Option<&light_client::indexer::OutputQueueDataV2>,
        input_queue: Option<&light_client::indexer::InputQueueDataV2>,
        on_chain_root: [u8; 32],
    ) -> Self {
        // Use the staging tree's current root, which includes all accumulated updates
        let staging_root = staging.current_root();
        tracing::debug!(
            "Reusing cached staging tree with {} accumulated updates (batch indices reset to 0 for new queue data), staging_root={:?}",
            staging.get_updates().len(),
            &staging_root[..8]
        );

        // CRITICAL: Merge fresh nodes from indexer into the cached staging tree.
        // The indexer contains new deduplicated nodes for the new queue elements.
        // The cached staging tree needs these fresh nodes to generate correct proofs for new batches.
        if let Err(e) =
            staging.merge_fresh_nodes_from_indexer(output_queue, input_queue, on_chain_root)
        {
            tracing::error!(
                "Failed to merge fresh nodes into cached staging tree: {:?}",
                e
            );
        }

        Self {
            append_batch_index: 0,
            nullify_batch_index: 0,
            append_leaf_indices,
            staging,
        }
    }
}
