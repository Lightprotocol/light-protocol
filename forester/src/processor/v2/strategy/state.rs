use crate::processor::v2::{
    common::{batch_range, get_leaves_hashchain},
    helpers::{fetch_batches, fetch_onchain_state_root, fetch_zkp_batch_size},
    proof_worker::ProofInput,
    strategy::{CircuitType, QueueData, TreeStrategy},
    BatchContext, QueueWork,
};
use anyhow::anyhow;
use async_trait::async_trait;
use forester_utils::{
    staging_tree::{BatchType, StagingTree},
    utils::wait_for_indexer,
};
use light_batched_merkle_tree::constants::DEFAULT_BATCH_STATE_TREE_HEIGHT;
use light_client::rpc::Rpc;
use light_compressed_account::QueueType;
use light_prover_client::proof_types::{
    batch_append::BatchAppendsCircuitInputs, batch_update::BatchUpdateCircuitInputs,
};
use tracing::{debug, warn};

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

        // Retry loop: fetch from indexer, validate root, wait if mismatch
        const MAX_ROOT_RETRIES: u32 = 3;
        // let state_queue = None;

        // for attempt in 0..MAX_ROOT_RETRIES {
        let state_queue =
            match fetch_batches(context, None, None, fetch_len, zkp_batch_size).await? {
                Some(sq) => sq,
                None => return Ok(None),
            };

        //     // Validate indexer root matches on-chain root before generating proofs
        //     let onchain_root = fetch_onchain_state_root(context).await?;
        //     if sq.initial_root == onchain_root {
        //         state_queue = Some(sq);
        //         break;
        //     }

        //     warn!(
        //         "Indexer root mismatch for tree {} (attempt {}/{}): indexer={}, onchain={}. Waiting for indexer...",
        //         context.merkle_tree,
        //         attempt + 1,
        //         MAX_ROOT_RETRIES,
        //         bs58::encode(&sq.initial_root).into_string(),
        //         bs58::encode(&onchain_root).into_string()
        //     );

        //     // Wait for indexer to catch up
        //     let rpc = context.rpc_pool.get_connection().await?;
        //     if let Err(e) = wait_for_indexer(&*rpc).await {
        //         warn!("wait_for_indexer failed: {}", e);
        //     }
        // }

        // let state_queue = match state_queue {
        //     Some(sq) => sq,
        //     None => {
        //         warn!(
        //             "Indexer root still mismatched after {} retries for tree {}. Skipping.",
        //             MAX_ROOT_RETRIES,
        //             context.merkle_tree
        //         );
        //         return Ok(None);
        //     }
        // };

        let phase = match queue_work.queue_type {
            QueueType::OutputStateV2 => StatePhase::Append,
            QueueType::InputStateV2 => StatePhase::Nullify,
            _ => return Ok(None),
        };

        let initial_root = state_queue.initial_root;
        let root_seq = state_queue.root_seq;
        let nodes = &state_queue.nodes;
        let node_hashes = &state_queue.node_hashes;

        let (leaf_indices, leaves, next_index, available) = match phase {
            StatePhase::Append => {
                let batch = match state_queue.output_queue.as_ref() {
                    Some(b) if !b.leaf_indices.is_empty() => b,
                    _ => return Ok(None),
                };
                (
                    batch.leaf_indices.clone(),
                    batch.old_leaves.clone(),
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
                    batch.leaf_indices.clone(),
                    batch.current_leaves.clone(),
                    None,
                    batch.leaf_indices.len(),
                )
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
