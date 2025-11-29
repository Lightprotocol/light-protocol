/// Tree-Specific Processing Strategies
///
/// Concrete implementations of `TreeStrategy` for state and address trees.

use anyhow::anyhow;
use async_trait::async_trait;
use forester_utils::{fast_address_staging_tree::FastAddressStagingTree, staging_tree::StagingTree};
use light_batched_merkle_tree::constants::DEFAULT_BATCH_STATE_TREE_HEIGHT;
use light_client::rpc::Rpc;
use light_compressed_account::QueueType;
use light_prover_client::proof_types::{
    batch_append::BatchAppendsCircuitInputs, batch_update::BatchUpdateCircuitInputs,
};
use std::time::Instant;
use tracing::{debug, info};

use crate::processor::v2::{
    common::{batch_range, get_leaves_hashchain},
    state::{
        helpers::{
            fetch_address_batches, fetch_address_zkp_batch_size, fetch_batches,
            fetch_zkp_batch_size,
        },
        proof_worker::ProofInput,
    },
    unified::{CircuitType, QueueData, TreeStrategy},
    BatchContext, QueueWork,
};

use forester_utils::staging_tree::BatchType;

// ============================================================================
// State Tree Strategy
// ============================================================================

/// Strategy for state tree batch processing.
///
/// Handles both Append and Nullify phases for state trees.
#[derive(Debug, Clone)]
pub struct StateTreeStrategy;

/// Phase of state tree processing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatePhase {
    Append,
    Nullify,
}

/// State-specific queue data
#[derive(Debug)]
pub struct StateQueueData {
    pub staging_tree: StagingTree,
    pub state_queue: light_client::indexer::StateQueueDataV2,
    pub phase: StatePhase,
    pub next_index: Option<u64>,
}

#[async_trait]
impl<R: Rpc> TreeStrategy<R> for StateTreeStrategy {
    type StagingTree = StateQueueData;

    fn name(&self) -> &'static str {
        "State"
    }

    fn circuit_type(&self, queue_data: &Self::StagingTree) -> CircuitType {
        match queue_data.phase {
            StatePhase::Append => CircuitType::Append,
            StatePhase::Nullify => CircuitType::Nullify,
        }
    }

    async fn fetch_zkp_batch_size(&self, context: &BatchContext<R>) -> crate::Result<u64> {
        fetch_zkp_batch_size(context).await
    }

    async fn fetch_queue_data(
        &self,
        context: &BatchContext<R>,
        queue_work: &QueueWork,
        max_batches: usize,
        zkp_batch_size: u64,
    ) -> crate::Result<Option<QueueData<Self::StagingTree>>> {
        let zkp_batch_size_usize = zkp_batch_size as usize;
        let total_needed = max_batches.saturating_mul(zkp_batch_size_usize);
        let fetch_len = total_needed as u64;

        let state_queue = match fetch_batches(context, None, None, fetch_len, zkp_batch_size).await? {
            Some(sq) => sq,
            None => return Ok(None),
        };

        // Determine phase based on queue type
        let phase = match queue_work.queue_type {
            QueueType::OutputStateV2 => StatePhase::Append,
            QueueType::InputStateV2 => StatePhase::Nullify,
            _ => return Ok(None),
        };

        // Capture initial_root before moving state_queue
        let initial_root = state_queue.initial_root;
        let root_seq = state_queue.root_seq;
        let nodes = &state_queue.nodes;
        let node_hashes = &state_queue.node_hashes;

        // Get the appropriate batch data
        let (leaf_indices, leaves, next_index, available) = match phase {
            StatePhase::Append => {
                let batch = match state_queue.output_queue.as_ref() {
                    Some(b) if !b.leaf_indices.is_empty() => b,
                    _ => return Ok(None),
                };
                (
                    &batch.leaf_indices,
                    &batch.old_leaves,
                    Some(batch.next_index),
                    batch.leaf_indices.len(),
                )
            }
            StatePhase::Nullify => {
                let batch = match state_queue.input_queue.as_ref() {
                    Some(b) if !b.leaf_indices.is_empty() => b,
                    _ => return Ok(None),
                };
                (
                    &batch.leaf_indices,
                    &batch.current_leaves,
                    None,
                    batch.leaf_indices.len(),
                )
            }
        };

        // Build staging tree
        let staging_tree = StagingTree::new(
            leaf_indices,
            leaves,
            nodes,
            node_hashes,
            initial_root,
            root_seq,
            DEFAULT_BATCH_STATE_TREE_HEIGHT as usize,
        )?;

        let num_batches = (available / zkp_batch_size_usize).min(max_batches);
        if num_batches == 0 {
            return Ok(None);
        }

        Ok(Some(QueueData {
            staging_tree: StateQueueData {
                staging_tree,
                state_queue,
                phase,
                next_index,
            },
            initial_root,
            num_batches,
        }))
    }

    fn build_proof_job(
        &self,
        queue_data: &mut Self::StagingTree,
        batch_idx: usize,
        start: usize,
        zkp_batch_size: u64,
    ) -> crate::Result<(ProofInput, [u8; 32])> {
        match queue_data.phase {
            StatePhase::Append => {
                self.build_append_job(queue_data, batch_idx, start, zkp_batch_size)
            }
            StatePhase::Nullify => {
                self.build_nullify_job(queue_data, batch_idx, start, zkp_batch_size)
            }
        }
    }

    fn validate_queue_work(&self, queue_work: &QueueWork) -> bool {
        matches!(
            queue_work.queue_type,
            QueueType::OutputStateV2 | QueueType::InputStateV2
        )
    }
}

impl StateTreeStrategy {
    
    fn build_append_job(
        &self,
        queue_data: &mut StateQueueData,
        batch_idx: usize,
        start: usize,
        zkp_batch_size: u64,
    ) -> crate::Result<(ProofInput, [u8; 32])> {
        let batch = queue_data
            .state_queue
            .output_queue
            .as_ref()
            .ok_or_else(|| anyhow!("Output queue not present"))?;

        let range = batch_range(zkp_batch_size, batch.account_hashes.len(), start);
        let leaves = batch.account_hashes[range.clone()].to_vec();
        let leaf_indices = batch.leaf_indices[range].to_vec();

        let hashchain_idx = start / zkp_batch_size as usize;
        let batch_seq = queue_data.state_queue.root_seq + (batch_idx as u64) + 1;

        let result = queue_data.staging_tree.process_batch_updates(
            &leaf_indices,
            &leaves,
            BatchType::Append,
            batch_idx,
            batch_seq,
        )?;

        let new_root = result.new_root;
        let leaves_hashchain = get_leaves_hashchain(&batch.leaves_hash_chains, hashchain_idx)?;
        let start_index = leaf_indices.first().copied().unwrap_or(0) as u32;

        let circuit_inputs =
            BatchAppendsCircuitInputs::new::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                result.into(),
                start_index,
                leaves,
                leaves_hashchain,
                zkp_batch_size as u32,
            )
            .map_err(|e| anyhow!("Failed to build append inputs: {}", e))?;

        Ok((ProofInput::Append(circuit_inputs), new_root))
    }

    fn build_nullify_job(
        &self,
        queue_data: &mut StateQueueData,
        batch_idx: usize,
        start: usize,
        zkp_batch_size: u64,
    ) -> crate::Result<(ProofInput, [u8; 32])> {
        let batch = queue_data
            .state_queue
            .input_queue
            .as_ref()
            .ok_or_else(|| anyhow!("Input queue not present"))?;

        let range = batch_range(zkp_batch_size, batch.account_hashes.len(), start);
        let account_hashes = batch.account_hashes[range.clone()].to_vec();
        let tx_hashes = batch.tx_hashes[range.clone()].to_vec();
        let nullifiers = batch.nullifiers[range.clone()].to_vec();
        let leaf_indices = batch.leaf_indices[range].to_vec();

        let hashchain_idx = start / zkp_batch_size as usize;
        let batch_seq = queue_data.state_queue.root_seq + (batch_idx as u64) + 1;

        let result = queue_data.staging_tree.process_batch_updates(
            &leaf_indices,
            &nullifiers,
            BatchType::Nullify,
            batch_idx,
            batch_seq,
        )?;

        info!(
            "nullify batch {} root {:?}[..4] => {:?}[..4]",
            batch_idx,
            &result.old_root[..4],
            &result.new_root[..4]
        );

        let new_root = result.new_root;
        let leaves_hashchain = get_leaves_hashchain(&batch.leaves_hash_chains, hashchain_idx)?;
        let path_indices: Vec<u32> = leaf_indices.iter().map(|idx| *idx as u32).collect();

        let circuit_inputs =
            BatchUpdateCircuitInputs::new::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                result.into(),
                tx_hashes,
                account_hashes,
                leaves_hashchain,
                path_indices,
                zkp_batch_size as u32,
            )
            .map_err(|e| anyhow!("Failed to build nullify inputs: {}", e))?;

        Ok((ProofInput::Nullify(circuit_inputs), new_root))
    }
}

// ============================================================================
// Address Tree Strategy
// ============================================================================

/// Strategy for address tree batch processing.
///
/// Handles Append operations for address trees.
#[derive(Debug, Clone)]
pub struct AddressTreeStrategy;

/// Address-specific queue data
#[derive(Debug)]
pub struct AddressQueueData {
    pub staging_tree: FastAddressStagingTree,
    pub address_queue: light_client::indexer::AddressQueueDataV2,
}

#[async_trait]
impl<R: Rpc> TreeStrategy<R> for AddressTreeStrategy {
    type StagingTree = AddressQueueData;

    fn name(&self) -> &'static str {
        "Address"
    }

    fn circuit_type(&self, _queue_data: &Self::StagingTree) -> CircuitType {
        CircuitType::AddressAppend
    }

    async fn fetch_zkp_batch_size(&self, context: &BatchContext<R>) -> crate::Result<u64> {
        fetch_address_zkp_batch_size(context).await
    }

    async fn fetch_queue_data(
        &self,
        context: &BatchContext<R>,
        _queue_work: &QueueWork,
        max_batches: usize,
        zkp_batch_size: u64,
    ) -> crate::Result<Option<QueueData<Self::StagingTree>>> {
        let zkp_batch_size_usize = zkp_batch_size as usize;
        let total_needed = max_batches.saturating_mul(zkp_batch_size_usize);
        let fetch_len = total_needed as u64;

        debug!(
            "Fetching address batches: fetch_len={}, zkp_batch_size={}",
            fetch_len, zkp_batch_size
        );

        let address_queue = match fetch_address_batches(context, None, fetch_len, zkp_batch_size).await? {
            Some(aq) if !aq.addresses.is_empty() => aq,
            Some(_) => {
                debug!("Address queue is empty");
                return Ok(None);
            }
            None => {
                debug!("No address queue data available");
                return Ok(None);
            }
        };

        // Validate required data
        if address_queue.subtrees.is_empty() {
            return Err(anyhow!("Address queue missing subtrees data"));
        }

        // Calculate batches
        let available = address_queue.addresses.len();
        let num_batches = (available / zkp_batch_size_usize).min(max_batches);

        if num_batches == 0 {
            debug!(
                "Not enough addresses for a complete batch: have {}, need {}",
                available, zkp_batch_size_usize
            );
            return Ok(None);
        }

        // Validate hash chains
        if address_queue.leaves_hash_chains.len() < num_batches {
            return Err(anyhow!(
                "Insufficient leaves_hash_chains: have {}, need {}",
                address_queue.leaves_hash_chains.len(),
                num_batches
            ));
        }

        // Capture initial_root before moving address_queue
        let initial_root = address_queue.initial_root;
        let start_index = address_queue.start_index;
        let nodes_len = address_queue.nodes.len();

        // Build staging tree using nodes for direct proof lookups
        let staging_tree = if !address_queue.nodes.is_empty() {
            FastAddressStagingTree::from_nodes(
                &address_queue.nodes,
                &address_queue.node_hashes,
                initial_root,
                start_index as usize,
            )?
        } else {
            // Fallback to subtrees-only mode (no direct proofs)
            FastAddressStagingTree::from_subtrees(
                address_queue.subtrees.to_vec(),
                start_index as usize,
                initial_root,
            )?
        };

        info!(
            "Synced address tree: root {:?}[..4], start_index {}, {} nodes",
            &initial_root[..4],
            start_index,
            nodes_len
        );

        Ok(Some(QueueData {
            staging_tree: AddressQueueData {
                staging_tree,
                address_queue,
            },
            initial_root,
            num_batches,
        }))
    }

    fn build_proof_job(
        &self,
        queue_data: &mut Self::StagingTree,
        batch_idx: usize,
        start: usize,
        zkp_batch_size: u64,
    ) -> crate::Result<(ProofInput, [u8; 32])> {
        let address_queue = &queue_data.address_queue;
        let range = batch_range(zkp_batch_size, address_queue.addresses.len(), start);
        let addresses = address_queue.addresses[range.clone()].to_vec();
        let zkp_batch_size_actual = addresses.len();

        // Get batch data
        let low_element_values = address_queue.low_element_values[range.clone()].to_vec();
        let low_element_next_values =
            address_queue.low_element_next_values[range.clone()].to_vec();
        let low_element_indices: Vec<usize> = address_queue.low_element_indices[range.clone()]
            .iter()
            .map(|&i| i as usize)
            .collect();
        let low_element_next_indices: Vec<usize> =
            address_queue.low_element_next_indices[range.clone()]
                .iter()
                .map(|&i| i as usize)
                .collect();
        let low_element_proofs = address_queue.low_element_proofs[range].to_vec();

        // Get hash chain
        let leaves_hashchain = get_leaves_hashchain(&address_queue.leaves_hash_chains, batch_idx)?;

        // Process batch - measure time for performance analysis
        let batch_start = Instant::now();
        let result = queue_data
            .staging_tree
            .process_batch(
                addresses,
                low_element_values,
                low_element_next_values,
                low_element_indices,
                low_element_next_indices,
                low_element_proofs,
                leaves_hashchain,
                zkp_batch_size_actual,
            )
            .map_err(|e| anyhow!("Failed to process address batch: {}", e))?;
        let batch_duration = batch_start.elapsed();

        let new_root = result.new_root;
        info!(
            "Address batch {} circuit inputs: {:?} (root {:?}[..4] -> {:?}[..4])",
            batch_idx,
            batch_duration,
            &result.old_root[..4],
            &new_root[..4]
        );

        Ok((ProofInput::AddressAppend(result.circuit_inputs), new_root))
    }

    fn validate_queue_work(&self, _queue_work: &QueueWork) -> bool {
        // Address trees always process from the queue
        true
    }
}
