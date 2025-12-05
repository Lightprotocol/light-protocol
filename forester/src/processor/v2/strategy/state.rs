use crate::processor::v2::{
    common::{batch_range, get_leaves_hashchain},
    helpers::{fetch_batches, fetch_onchain_state_root, fetch_zkp_batch_size},
    proof_worker::ProofInput,
    strategy::{CircuitType, QueueData, TreeStrategy},
    BatchContext, QueueWork,
};
use anyhow::anyhow;
use async_trait::async_trait;
use forester_utils::staging_tree::{BatchType, StagingTree};
use light_batched_merkle_tree::constants::DEFAULT_BATCH_STATE_TREE_HEIGHT;
use light_client::rpc::Rpc;
use light_prover_client::proof_types::{
    batch_append::BatchAppendsCircuitInputs, batch_update::BatchUpdateCircuitInputs,
};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct StateTreeStrategy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatePhase {
    Append,
    Nullify,
}

#[derive(Debug)]
pub struct StateQueueData {
    pub staging_tree: StagingTree,
    pub state_queue: light_client::indexer::StateQueueDataV2,
    pub phase: StatePhase,
    pub next_index: Option<u64>,
    /// Number of APPEND batches that must be processed before NULLIFY batches.
    /// This is used when both output and input queues have data to ensure
    /// proper root chaining: initial_root -> post_append_root -> post_nullify_root
    pub append_batches_before_nullify: usize,
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

    fn circuit_type_for_batch(&self, queue_data: &Self::StagingTree, batch_idx: usize) -> CircuitType {
        // When processing combined APPEND+NULLIFY, determine circuit type based on batch index
        let is_append_phase = batch_idx < queue_data.append_batches_before_nullify
            || (queue_data.append_batches_before_nullify == 0
                && queue_data.phase == StatePhase::Append);

        if is_append_phase {
            CircuitType::Append
        } else {
            CircuitType::Nullify
        }
    }

    async fn fetch_zkp_batch_size(&self, context: &BatchContext<R>) -> crate::Result<u64> {
        fetch_zkp_batch_size(context).await
    }

    async fn fetch_onchain_root(&self, context: &BatchContext<R>) -> crate::Result<[u8; 32]> {
        fetch_onchain_state_root(context).await
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

        let state_queue =
            match fetch_batches(context, None, None, fetch_len, zkp_batch_size).await? {
                Some(sq) => sq,
                None => return Ok(None),
            };

        // Ignore queue_work.queue_type hint - always check both queues and process
        // APPEND batches before NULLIFY batches to ensure proper root chaining.
        let _ = queue_work.queue_type;

        let initial_root = state_queue.initial_root;
        let root_seq = state_queue.root_seq;
        let nodes = &state_queue.nodes;
        let node_hashes = &state_queue.node_hashes;

        // Count available items for both phases
        let append_items = state_queue
            .output_queue
            .as_ref()
            .map(|oq| oq.leaf_indices.len())
            .unwrap_or(0);
        let nullify_items = state_queue
            .input_queue
            .as_ref()
            .map(|iq| iq.leaf_indices.len())
            .unwrap_or(0);

        let append_batches = append_items / zkp_batch_size_usize;
        let nullify_batches = nullify_items / zkp_batch_size_usize;

        // Always process APPEND batches first, then NULLIFY batches.
        // This ensures proper root chaining: initial_root -> post_append_root -> post_nullify_root
        let (append_batches_before_nullify, total_batches, effective_phase) =
            if append_batches > 0 && nullify_batches > 0 {
                // Process both: APPENDs first, then NULLIFYs
                let total = (append_batches + nullify_batches).min(max_batches);
                let appends_to_process = append_batches.min(total);
                debug!(
                    "Processing {} APPEND batches then {} NULLIFY batches (total: {})",
                    appends_to_process,
                    total.saturating_sub(appends_to_process),
                    total
                );
                (appends_to_process, total, StatePhase::Append)
            } else if append_batches > 0 {
                (0, append_batches.min(max_batches), StatePhase::Append)
            } else if nullify_batches > 0 {
                (0, nullify_batches.min(max_batches), StatePhase::Nullify)
            } else {
                return Ok(None);
            };

        // Get data for staging tree initialization.
        // We need to include leaf data for BOTH phases if we're processing both,
        // since the staging tree needs all leaves to compute proofs.
        let (leaf_indices, leaves, next_index) = if append_batches_before_nullify > 0 {
            // Processing both APPEND and NULLIFY - combine leaf data
            let output_batch = state_queue.output_queue.as_ref().unwrap();
            let input_batch = state_queue.input_queue.as_ref().unwrap();

            let mut combined_indices = output_batch.leaf_indices.clone();
            let mut combined_leaves = output_batch.old_leaves.clone();

            // Add NULLIFY leaves (these are at different indices)
            combined_indices.extend(input_batch.leaf_indices.iter().copied());
            combined_leaves.extend(input_batch.current_leaves.iter().copied());

            (combined_indices, combined_leaves, Some(output_batch.next_index))
        } else {
            match effective_phase {
                StatePhase::Append => {
                    let batch = state_queue.output_queue.as_ref().unwrap();
                    (
                        batch.leaf_indices.clone(),
                        batch.old_leaves.clone(),
                        Some(batch.next_index),
                    )
                }
                StatePhase::Nullify => {
                    let batch = state_queue.input_queue.as_ref().unwrap();
                    (batch.leaf_indices.clone(), batch.current_leaves.clone(), None)
                }
            }
        };

        // Move CPU-bound tree initialization to blocking thread pool
        // to avoid blocking the async executor
        let nodes = nodes.clone();
        let node_hashes = node_hashes.clone();
        let staging_tree = tokio::task::spawn_blocking(move || {
            let start = std::time::Instant::now();
            let tree = StagingTree::new(
                &leaf_indices,
                &leaves,
                &nodes,
                &node_hashes,
                initial_root,
                root_seq,
                DEFAULT_BATCH_STATE_TREE_HEIGHT as usize,
            );
            debug!(
                "StagingTree init took {:?}, leaves={}, nodes={}",
                start.elapsed(),
                leaf_indices.len(),
                nodes.len()
            );
            tree
        })
        .await
        .map_err(|e| anyhow!("spawn_blocking join error: {}", e))??;

        if total_batches == 0 {
            return Ok(None);
        }

        Ok(Some(QueueData {
            staging_tree: StateQueueData {
                staging_tree,
                state_queue,
                phase: effective_phase,
                next_index,
                append_batches_before_nullify,
            },
            initial_root,
            num_batches: total_batches,
        }))
    }

    fn build_proof_job(
        &self,
        queue_data: &mut Self::StagingTree,
        batch_idx: usize,
        start: usize,
        zkp_batch_size: u64,
        epoch: u64,
        tree: &str,
    ) -> crate::Result<(ProofInput, [u8; 32])> {
        // When we have both APPEND and NULLIFY batches, process APPENDs first.
        // The append_batches_before_nullify field tells us how many APPEND batches
        // must be processed before we switch to NULLIFY.
        let is_append_phase = batch_idx < queue_data.append_batches_before_nullify
            || (queue_data.append_batches_before_nullify == 0
                && queue_data.phase == StatePhase::Append);

        if is_append_phase {
            self.build_append_job(queue_data, batch_idx, start, zkp_batch_size, epoch, tree)
        } else {
            // Adjust batch_idx and start for NULLIFY phase
            let nullify_batch_idx = batch_idx - queue_data.append_batches_before_nullify;
            let nullify_start = nullify_batch_idx * zkp_batch_size as usize;
            self.build_nullify_job(queue_data, nullify_batch_idx, nullify_start, zkp_batch_size, epoch, tree)
        }
    }
}

impl StateTreeStrategy {
    fn build_append_job(
        &self,
        queue_data: &mut StateQueueData,
        batch_idx: usize,
        start: usize,
        zkp_batch_size: u64,
        epoch: u64,
        tree: &str,
    ) -> crate::Result<(ProofInput, [u8; 32])> {
        let batch = queue_data
            .state_queue
            .output_queue
            .as_ref()
            .ok_or_else(|| anyhow!("Output queue not present"))?;

        let range = batch_range(zkp_batch_size, batch.account_hashes.len(), start);
        let leaves = &batch.account_hashes[range.clone()];
        let leaf_indices = &batch.leaf_indices[range];

        let hashchain_idx = start / zkp_batch_size as usize;
        let batch_seq = queue_data.state_queue.root_seq + (batch_idx as u64) + 1;

        let result = queue_data.staging_tree.process_batch_updates(
            &leaf_indices,
            &leaves,
            BatchType::Append,
            batch_idx,
            batch_seq,
            epoch,
            tree,
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
        epoch: u64,
        tree: &str,
    ) -> crate::Result<(ProofInput, [u8; 32])> {
        let batch = queue_data
            .state_queue
            .input_queue
            .as_ref()
            .ok_or_else(|| anyhow!("Input queue not present"))?;

        let range = batch_range(zkp_batch_size, batch.account_hashes.len(), start);
        let account_hashes = &batch.account_hashes[range.clone()];
        let tx_hashes = &batch.tx_hashes[range.clone()];
        let nullifiers = &batch.nullifiers[range.clone()];
        let leaf_indices = &batch.leaf_indices[range];

        let hashchain_idx = start / zkp_batch_size as usize;
        let batch_seq = queue_data.state_queue.root_seq + (batch_idx as u64) + 1;

        let result = queue_data.staging_tree.process_batch_updates(
            leaf_indices,
            nullifiers,
            BatchType::Nullify,
            batch_idx,
            batch_seq,
            epoch,
            tree,
        )?;

        let new_root = result.new_root;
        let leaves_hashchain = get_leaves_hashchain(&batch.leaves_hash_chains, hashchain_idx)?;
        let path_indices: Vec<u32> = leaf_indices.iter().map(|idx| *idx as u32).collect();

        let circuit_inputs =
            BatchUpdateCircuitInputs::new::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                result.into(),
                tx_hashes,
                account_hashes,
                leaves_hashchain,
                &path_indices,
                zkp_batch_size as u32,
            )
            .map_err(|e| anyhow!("Failed to build nullify inputs: {}", e))?;

        Ok((ProofInput::Nullify(circuit_inputs), new_root))
    }
}
