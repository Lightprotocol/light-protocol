use anyhow::anyhow;
use async_trait::async_trait;
use forester_utils::staging_tree::{BatchType, StagingTree};
use light_batched_merkle_tree::constants::DEFAULT_BATCH_STATE_TREE_HEIGHT;
use light_client::rpc::Rpc;
use light_prover_client::proof_types::{
    batch_append::BatchAppendsCircuitInputs, batch_update::BatchUpdateCircuitInputs,
};
use tracing::{debug, instrument};

use crate::processor::v2::{
    batch_job_builder::BatchJobBuilder,
    common::{batch_range, get_leaves_hashchain},
    helpers::{fetch_onchain_state_root, fetch_paginated_batches, fetch_zkp_batch_size},
    proof_worker::ProofInput,
    root_guard::{reconcile_alignment, AlignmentDecision},
    strategy::{CircuitType, QueueData, TreeStrategy},
    BatchContext, QueueWork,
};

#[derive(Debug, Clone)]
pub struct StateTreeStrategy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatePhase {
    Append,
    Nullify,
}

#[derive(Debug, Clone, Copy)]
pub enum BatchOp {
    Append(usize),
    Nullify(usize),
}

#[derive(Debug)]
pub struct StateQueueData {
    pub staging_tree: StagingTree,
    pub state_queue: light_client::indexer::StateQueueData,
    pub phase: StatePhase,
    pub next_index: Option<u64>,
    pub append_batches_before_nullify: usize,
    pub interleaved_ops: Option<Vec<BatchOp>>,
    /// First queue index for output queue data (where this batch starts)
    pub output_first_queue_index: u64,
    /// First queue index for input queue data (where this batch starts)
    pub input_first_queue_index: u64,
    /// Number of output queue elements processed (for alignment tracking)
    pub output_processed: usize,
    /// Number of input queue elements processed (for alignment tracking)
    pub input_processed: usize,
    /// ZKP batch size for alignment calculations
    pub zkp_batch_size: usize,
}

impl StateQueueData {
    /// Get number of remaining output batches
    pub fn remaining_output_batches(&self) -> usize {
        let total_output = self
            .state_queue
            .output_queue
            .as_ref()
            .map(|oq| oq.leaf_indices.len())
            .unwrap_or(0);
        let remaining = total_output.saturating_sub(self.output_processed);
        remaining / self.zkp_batch_size
    }

    /// Get number of remaining input batches
    pub fn remaining_input_batches(&self) -> usize {
        let total_input = self
            .state_queue
            .input_queue
            .as_ref()
            .map(|iq| iq.leaf_indices.len())
            .unwrap_or(0);
        let remaining = total_input.saturating_sub(self.input_processed);
        remaining / self.zkp_batch_size
    }

    /// Get expected next output queue index (for alignment validation when re-fetching)
    pub fn expected_output_queue_index(&self) -> u64 {
        self.output_first_queue_index + self.output_processed as u64
    }

    /// Get expected next input queue index (for alignment validation when re-fetching)
    pub fn expected_input_queue_index(&self) -> u64 {
        self.input_first_queue_index + self.input_processed as u64
    }
}

use light_compressed_account::QueueType;

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

    fn queue_type() -> QueueType {
        QueueType::OutputStateV2
    }

    fn circuit_type_for_batch(
        &self,
        queue_data: &Self::StagingTree,
        batch_idx: usize,
    ) -> CircuitType {
        if let Some(ref ops) = queue_data.interleaved_ops {
            if let Some(op) = ops.get(batch_idx) {
                return match op {
                    BatchOp::Append(_) => CircuitType::Append,
                    BatchOp::Nullify(_) => CircuitType::Nullify,
                };
            }
        }

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

    #[instrument(level = "debug", skip(self, context, queue_work), fields(tree = %context.merkle_tree))]
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

        let state_queue = match fetch_paginated_batches(context, fetch_len, zkp_batch_size).await? {
            Some(sq) => sq,
            None => return Ok(None),
        };

        let _ = queue_work.queue_type;

        let initial_root = state_queue.initial_root;
        let root_seq = state_queue.root_seq;
        let nodes = &state_queue.nodes;
        let node_hashes = &state_queue.node_hashes;

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

        debug!(
            append_items,
            nullify_items,
            output_queue = state_queue.output_queue.is_some(),
            input_queue = state_queue.input_queue.is_some(),
            "Queue data fetched"
        );

        let append_batches = append_items / zkp_batch_size_usize;
        let nullify_batches = nullify_items / zkp_batch_size_usize;

        let (append_batches_before_nullify, total_batches, effective_phase) =
            if append_batches > 0 && nullify_batches > 0 {
                let total = (append_batches + nullify_batches).min(max_batches);
                let half_batches = max_batches / 2;
                let appends_to_process = append_batches.min(half_batches).max(1);
                let nullifies_to_process =
                    nullify_batches.min(total.saturating_sub(appends_to_process));
                let actual_total = appends_to_process + nullifies_to_process;
                debug!(
                    "Processing {} APPEND batches then {} NULLIFY batches (total: {})",
                    appends_to_process, nullifies_to_process, actual_total
                );
                (appends_to_process, actual_total, StatePhase::Append)
            } else if append_batches > 0 {
                (0, append_batches.min(max_batches), StatePhase::Append)
            } else if nullify_batches > 0 {
                (0, nullify_batches.min(max_batches), StatePhase::Nullify)
            } else {
                return Ok(None);
            };

        let (leaf_indices, leaves, next_index) =
            if append_batches_before_nullify > 0 {
                let output_batch = state_queue.output_queue.as_ref().ok_or_else(|| {
                    anyhow!("Expected output_queue batch when processing appends")
                })?;
                let input_batch = state_queue.input_queue.as_ref().ok_or_else(|| {
                    anyhow!("Expected input_queue batch when processing nullifies")
                })?;

                let mut combined_indices = output_batch.leaf_indices.clone();
                let mut combined_leaves = output_batch.old_leaves.clone();

                combined_indices.extend(input_batch.leaf_indices.iter().copied());
                combined_leaves.extend(input_batch.current_leaves.iter().copied());

                (
                    combined_indices,
                    combined_leaves,
                    Some(output_batch.next_index),
                )
            } else {
                match effective_phase {
                    StatePhase::Append => {
                        let batch = state_queue.output_queue.as_ref().ok_or_else(|| {
                            anyhow!("Expected output_queue batch when processing appends")
                        })?;
                        (
                            batch.leaf_indices.clone(),
                            batch.old_leaves.clone(),
                            Some(batch.next_index),
                        )
                    }
                    StatePhase::Nullify => {
                        let batch = state_queue.input_queue.as_ref().ok_or_else(|| {
                            anyhow!("Expected input_queue batch when processing nullifies")
                        })?;
                        (
                            batch.leaf_indices.clone(),
                            batch.current_leaves.clone(),
                            None,
                        )
                    }
                }
            };

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

        let interleaved_ops = if append_batches_before_nullify > 0 {
            let output_batch = state_queue.output_queue.as_ref().ok_or_else(|| {
                anyhow!("Expected output_queue batch when computing interleaving ops")
            })?;
            let input_batch = state_queue.input_queue.as_ref().ok_or_else(|| {
                anyhow!("Expected input_queue batch when computing interleaving ops")
            })?;
            let initial_next_index = output_batch.next_index;

            let nullifies_to_process = total_batches.saturating_sub(append_batches_before_nullify);

            tracing::info!(
                "Interleave check: initial_next_index={}, nullify leaf_indices[0..min(10,len)]={:?}, batch_size={}, num_appends={}, num_nullifies={}",
                initial_next_index,
                &input_batch.leaf_indices[..input_batch.leaf_indices.len().min(10)],
                zkp_batch_size,
                append_batches_before_nullify,
                nullifies_to_process
            );

            Some(compute_interleaved_ops(
                append_batches_before_nullify,
                nullifies_to_process,
                initial_next_index,
                zkp_batch_size,
                &input_batch.leaf_indices,
            ))
        } else {
            None
        };

        let interleaved_total = interleaved_ops
            .as_ref()
            .map(|ops| ops.len())
            .unwrap_or(total_batches);
        if let Some(ops) = interleaved_ops.as_ref() {
            tracing::info!(
                "Interleaved ops: {} total ({} append, {} nullify)",
                interleaved_total,
                ops.iter()
                    .filter(|op| matches!(op, BatchOp::Append(_)))
                    .count(),
                ops.iter()
                    .filter(|op| matches!(op, BatchOp::Nullify(_)))
                    .count(),
            );
        }

        let output_first_queue_index = state_queue
            .output_queue
            .as_ref()
            .map(|oq| oq.first_queue_index)
            .unwrap_or(0);
        let input_first_queue_index = state_queue
            .input_queue
            .as_ref()
            .map(|iq| iq.first_queue_index)
            .unwrap_or(0);

        tracing::info!(
            "State queue ready: output_first_queue_index={}, input_first_queue_index={}, batches={}",
            output_first_queue_index, input_first_queue_index, interleaved_total
        );

        Ok(Some(QueueData {
            staging_tree: StateQueueData {
                staging_tree,
                state_queue,
                phase: effective_phase,
                next_index,
                append_batches_before_nullify,
                interleaved_ops,
                output_first_queue_index,
                input_first_queue_index,
                output_processed: 0,
                input_processed: 0,
                zkp_batch_size: zkp_batch_size_usize,
            },
            initial_root,
            num_batches: interleaved_total,
        }))
    }
}

impl StateQueueData {
    fn build_append_job(
        &mut self,
        batch_idx: usize,
        start: usize,
        zkp_batch_size: u64,
        epoch: u64,
        tree: &str,
    ) -> crate::Result<Option<(ProofInput, [u8; 32])>> {
        let zkp_batch_size_usize = zkp_batch_size as usize;
        let expected_queue_index = self.output_first_queue_index as usize + self.output_processed;
        let data_start_index = self.output_first_queue_index as usize;

        match reconcile_alignment(expected_queue_index, data_start_index, start) {
            AlignmentDecision::SkipOverlap => {
                let absolute_queue_index = data_start_index + start;
                tracing::debug!(
                    "Skipping output queue batch (overlap): absolute_index={}, expected_start={}",
                    absolute_queue_index,
                    expected_queue_index
                );
                return Ok(None);
            }
            AlignmentDecision::Gap | AlignmentDecision::StaleTree => {
                let absolute_queue_index = data_start_index + start;
                return Err(anyhow!(
                    "Output queue stale: expected start {}, got {}. Need to invalidate cache.",
                    expected_queue_index,
                    absolute_queue_index
                ));
            }
            AlignmentDecision::Process => {}
        }

        let batch = self
            .state_queue
            .output_queue
            .as_ref()
            .ok_or_else(|| anyhow!("Output queue not present"))?;

        let range = batch_range(zkp_batch_size, batch.account_hashes.len(), start);
        let leaves = &batch.account_hashes[range.clone()];
        let leaf_indices = &batch.leaf_indices[range];

        let hashchain_idx = start / zkp_batch_size_usize;
        let batch_seq = self.state_queue.root_seq + (batch_idx as u64) + 1;

        let result = self.staging_tree.process_batch_updates(
            leaf_indices,
            leaves,
            BatchType::Append,
            batch_idx,
            batch_seq,
            epoch,
            tree,
        )?;

        self.output_processed += zkp_batch_size_usize;

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

        Ok(Some((ProofInput::Append(circuit_inputs), new_root)))
    }

    fn build_nullify_job(
        &mut self,
        batch_idx: usize,
        start: usize,
        zkp_batch_size: u64,
        epoch: u64,
        tree: &str,
    ) -> crate::Result<Option<(ProofInput, [u8; 32])>> {
        let zkp_batch_size_usize = zkp_batch_size as usize;
        let expected_queue_index = self.input_first_queue_index as usize + self.input_processed;
        let data_start_index = self.input_first_queue_index as usize;

        match reconcile_alignment(expected_queue_index, data_start_index, start) {
            AlignmentDecision::SkipOverlap => {
                let absolute_queue_index = data_start_index + start;
                tracing::debug!(
                    "Skipping input queue batch (overlap): absolute_index={}, expected_start={}",
                    absolute_queue_index,
                    expected_queue_index
                );
                return Ok(None);
            }
            AlignmentDecision::Gap | AlignmentDecision::StaleTree => {
                let absolute_queue_index = data_start_index + start;
                return Err(anyhow!(
                    "Input queue stale: expected start {}, got {}. Need to invalidate cache.",
                    expected_queue_index,
                    absolute_queue_index
                ));
            }
            AlignmentDecision::Process => {}
        }

        let batch = self
            .state_queue
            .input_queue
            .as_ref()
            .ok_or_else(|| anyhow!("Input queue not present"))?;

        let range = batch_range(zkp_batch_size, batch.account_hashes.len(), start);
        let account_hashes = &batch.account_hashes[range.clone()];
        let tx_hashes = &batch.tx_hashes[range.clone()];
        let nullifiers = &batch.nullifiers[range.clone()];
        let leaf_indices = &batch.leaf_indices[range];

        let hashchain_idx = start / zkp_batch_size_usize;
        let batch_seq = self.state_queue.root_seq + (batch_idx as u64) + 1;

        let result = self.staging_tree.process_batch_updates(
            leaf_indices,
            nullifiers,
            BatchType::Nullify,
            batch_idx,
            batch_seq,
            epoch,
            tree,
        )?;

        self.input_processed += zkp_batch_size_usize;

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

        Ok(Some((ProofInput::Nullify(circuit_inputs), new_root)))
    }
}

impl BatchJobBuilder for StateQueueData {
    fn build_proof_job(
        &mut self,
        batch_idx: usize,
        zkp_batch_size: u64,
        epoch: u64,
        tree: &str,
    ) -> crate::Result<Option<(ProofInput, [u8; 32])>> {
        if let Some(ref ops) = self.interleaved_ops {
            if let Some(op) = ops.get(batch_idx) {
                return match op {
                    BatchOp::Append(append_idx) => {
                        let start = append_idx * zkp_batch_size as usize;
                        self.build_append_job(*append_idx, start, zkp_batch_size, epoch, tree)
                    }
                    BatchOp::Nullify(nullify_idx) => {
                        let start = nullify_idx * zkp_batch_size as usize;
                        self.build_nullify_job(*nullify_idx, start, zkp_batch_size, epoch, tree)
                    }
                };
            }
        }

        let is_append_phase = batch_idx < self.append_batches_before_nullify
            || (self.append_batches_before_nullify == 0 && self.phase == StatePhase::Append);

        if is_append_phase {
            let start = batch_idx * zkp_batch_size as usize;
            self.build_append_job(batch_idx, start, zkp_batch_size, epoch, tree)
        } else {
            let nullify_batch_idx = batch_idx - self.append_batches_before_nullify;
            let start = nullify_batch_idx * zkp_batch_size as usize;
            self.build_nullify_job(nullify_batch_idx, start, zkp_batch_size, epoch, tree)
        }
    }
}

fn compute_interleaved_ops(
    num_appends: usize,
    num_nullifies: usize,
    initial_next_index: u64,
    batch_size: u64,
    nullify_leaf_indices: &[u64],
) -> Vec<BatchOp> {
    let batch_size_usize = batch_size as usize;
    let mut ops = Vec::with_capacity(num_appends + num_nullifies);

    let mut appends_scheduled = 0usize;
    let mut nullifies_scheduled = 0usize;

    let nullify_batch_max_indices: Vec<u64> = (0..num_nullifies)
        .map(|batch_idx| {
            let start = batch_idx * batch_size_usize;
            let end = ((batch_idx + 1) * batch_size_usize).min(nullify_leaf_indices.len());
            nullify_leaf_indices[start..end]
                .iter()
                .copied()
                .max()
                .unwrap_or(0)
        })
        .collect();

    if !nullify_batch_max_indices.is_empty() {
        tracing::info!(
            "compute_interleaved_ops: nullify_batch_max_indices[0..min(5,len)]={:?}",
            &nullify_batch_max_indices[..nullify_batch_max_indices.len().min(5)]
        );
    }

    while appends_scheduled < num_appends || nullifies_scheduled < num_nullifies {
        if appends_scheduled < num_appends {
            ops.push(BatchOp::Append(appends_scheduled));
            appends_scheduled += 1;
        }

        let boundary = initial_next_index + (appends_scheduled as u64 * batch_size);
        let mut scheduled_this_round = 0;
        while nullifies_scheduled < num_nullifies {
            let max_leaf_idx = nullify_batch_max_indices[nullifies_scheduled];
            if max_leaf_idx < boundary {
                ops.push(BatchOp::Nullify(nullifies_scheduled));
                nullifies_scheduled += 1;
                scheduled_this_round += 1;
            } else {
                if nullifies_scheduled == 0 && appends_scheduled <= 2 {
                    tracing::info!(
                        "Nullify batch {} skipped: max_leaf_idx={} >= boundary={} (initial_next_index={}, appends_scheduled={})",
                        nullifies_scheduled, max_leaf_idx, boundary, initial_next_index, appends_scheduled
                    );
                }
                break;
            }
        }
        if scheduled_this_round > 0 && appends_scheduled <= 2 {
            tracing::info!(
                "After append {}: scheduled {} nullifies (boundary={})",
                appends_scheduled - 1,
                scheduled_this_round,
                boundary
            );
        }

        if appends_scheduled >= num_appends && nullifies_scheduled < num_nullifies {
            while nullifies_scheduled < num_nullifies {
                ops.push(BatchOp::Nullify(nullifies_scheduled));
                nullifies_scheduled += 1;
            }
        }
    }

    ops
}
