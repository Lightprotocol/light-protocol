use anyhow::anyhow;
use forester_utils::{
    staging_tree::{BatchType, StagingTree},
    utils::wait_for_indexer,
};
use kameo::{
    actor::{ActorRef, WeakActorRef},
    error::ActorStopReason,
    message::Message,
    Actor,
};
use light_batched_merkle_tree::constants::DEFAULT_BATCH_STATE_TREE_HEIGHT;
use light_client::rpc::Rpc;
use light_compressed_account::QueueType;
use light_prover_client::proof_types::{
    batch_append::BatchAppendsCircuitInputs, batch_update::BatchUpdateCircuitInputs,
};
use light_registry::protocol_config::state::EpochState;
use tokio::sync::mpsc;
use tracing::{debug, info, trace, warn};

use crate::processor::v2::{
    state::{
        helpers::{fetch_batches, fetch_zkp_batch_size},
        proof_worker::{spawn_proof_workers, ProofInput, ProofJob},
        tx_sender::TxSender,
    },
    BatchContext,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Append,
    Nullify,
}

#[derive(Debug, Clone)]
pub struct QueueWork {
    pub queue_type: QueueType,
    pub queue_size: u64,
}

#[derive(Debug, Clone)]
pub struct ProcessQueueUpdate {
    pub queue_work: QueueWork,
}

pub struct StateSupervisor<R: Rpc> {
    context: BatchContext<R>,
    staging_tree: Option<StagingTree>,
    current_root: [u8; 32],
    /// Tree's next_index for appends (updated after each append batch)
    next_index: u64,
    /// ZKP batch size fetched once from on-chain at startup
    zkp_batch_size: u64,
    seq: u64,
}

impl<R: Rpc> Actor for StateSupervisor<R> {
    type Args = BatchContext<R>;
    type Error = anyhow::Error;

    async fn on_start(
        context: Self::Args,
        _actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        info!(
            "StateSupervisor actor starting for tree {}",
            context.merkle_tree
        );

        // Fetch zkp_batch_size once from on-chain (this is static per tree)
        let zkp_batch_size = fetch_zkp_batch_size(&context).await?;
        info!(
            "StateSupervisor fetched zkp_batch_size={} for tree {}",
            zkp_batch_size, context.merkle_tree
        );

        // Try to restore staging tree from cache (will be validated against indexer data later)
        let staging_tree = {
            let cache = context.staging_tree_cache.lock().await;
            cache.clone()
        };

        Ok(Self {
            context,
            staging_tree,
            current_root: [0u8; 32],
            next_index: 0,
            zkp_batch_size,
            seq: 0,
        })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        info!(
            "StateSupervisor actor stopping for tree {}",
            self.context.merkle_tree
        );
        // Save staging tree to cache before stopping
        if let Some(ref tree) = self.staging_tree {
            let mut cache = self.context.staging_tree_cache.lock().await;
            *cache = Some(tree.clone());
        }
        Ok(())
    }
}

impl<R: Rpc> Message<ProcessQueueUpdate> for StateSupervisor<R> {
    type Reply = crate::Result<usize>;

    async fn handle(
        &mut self,
        msg: ProcessQueueUpdate,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.process_queue_update(msg.queue_work).await
    }
}

/// Common batch data needed for staging tree reconciliation.
struct StagingTreeBatchData<'a> {
    leaf_indices: &'a [u64],
    leaves: &'a [[u8; 32]],
    nodes: &'a [u64],
    node_hashes: &'a [[u8; 32]],
    initial_root: [u8; 32],
    root_seq: u64,
}

impl<R: Rpc> StateSupervisor<R> {
    fn zkp_batch_size(&self) -> u64 {
        self.zkp_batch_size
    }

    /// Saves the current staging tree to the cache.
    async fn save_staging_tree_cache(&self) {
        let mut cache = self.context.staging_tree_cache.lock().await;
        *cache = self.staging_tree.clone();
    }

    /// Gets the leaves hashchain for a batch, returning an error if not found.
    fn get_leaves_hashchain(
        leaves_hash_chains: &[[u8; 32]],
        batch_idx: usize,
    ) -> crate::Result<[u8; 32]> {
        leaves_hash_chains.get(batch_idx).copied().ok_or_else(|| {
            anyhow!(
                "Missing leaves_hash_chain for batch {} (available: {})",
                batch_idx,
                leaves_hash_chains.len()
            )
        })
    }

    /// Computes the slice range for a batch given total length and start index.
    fn batch_range(&self, total_len: usize, start: usize) -> std::ops::Range<usize> {
        let end = (start + self.zkp_batch_size as usize).min(total_len);
        start..end
    }

    /// Finalizes a proof job by updating state and returning the job.
    async fn finish_job(&mut self, new_root: [u8; 32], inputs: ProofInput) -> Option<ProofJob> {
        self.current_root = new_root;
        self.seq += 1;
        self.save_staging_tree_cache().await;
        Some(ProofJob {
            seq: self.seq - 1,
            inputs,
        })
    }

    /// Reconciles staging tree with indexer data.
    /// Returns true if staging tree is ready for use, false if we should skip this batch.
    fn reconcile_staging_tree(
        &mut self,
        batch_idx: usize,
        batch_data: &StagingTreeBatchData<'_>,
        batch_type: &str,
    ) -> bool {
        // Initialize staging tree from indexer data if none exists
        if self.staging_tree.is_none() {
            match StagingTree::new(
                batch_data.leaf_indices,
                batch_data.leaves,
                batch_data.nodes,
                batch_data.node_hashes,
                batch_data.initial_root,
                batch_data.root_seq,
            ) {
                Ok(tree) => {
                    self.staging_tree = Some(tree);
                }
                Err(e) => {
                    warn!(
                        "Failed to initialize {} staging tree from indexer data: {}. Retrying later.",
                        batch_type, e
                    );
                    return false;
                }
            }
        }

        // Validate staging tree root matches indexer data for first batch
        if batch_idx == 0
            && self
                .staging_tree
                .as_ref()
                .map(|t| t.current_root() != batch_data.initial_root)
                .unwrap_or(true)
        {
            let staging = self.staging_tree.as_ref();
            let staging_seq = staging.map(|t| t.base_seq()).unwrap_or(0);
            let indexer_seq = batch_data.root_seq;
            let pending_updates = staging
                .map(|t| t.get_updates().to_vec())
                .unwrap_or_default();

            let rebuild_result = if indexer_seq > staging_seq {
                // Indexer is ahead - we're behind, rebuild entirely from indexer
                info!(
                    "{} staging tree behind indexer (staging_seq={}, indexer_seq={}), rebuilding",
                    batch_type, staging_seq, indexer_seq
                );
                StagingTree::new(
                    batch_data.leaf_indices,
                    batch_data.leaves,
                    batch_data.nodes,
                    batch_data.node_hashes,
                    batch_data.initial_root,
                    batch_data.root_seq,
                )
            } else if !pending_updates.is_empty() {
                // We're ahead with pending updates - rebuild and replay
                info!(
                    "{} staging tree ahead (staging_seq={}, indexer_seq={}, {} pending), rebuilding and replaying",
                    batch_type, staging_seq, indexer_seq, pending_updates.len()
                );
                match StagingTree::new(
                    batch_data.leaf_indices,
                    batch_data.leaves,
                    batch_data.nodes,
                    batch_data.node_hashes,
                    batch_data.initial_root,
                    batch_data.root_seq,
                ) {
                    Ok(mut tree) => {
                        let replayed = tree.replay_pending_updates(&pending_updates);
                        info!(
                            "Replayed {}/{} pending {} updates",
                            replayed,
                            pending_updates.len(),
                            batch_type.to_lowercase()
                        );
                        Ok(tree)
                    }
                    Err(e) => Err(e),
                }
            } else {
                // No pending updates, just rebuild
                info!(
                    "{} staging tree stale (staging_seq={}, indexer_seq={}), rebuilding",
                    batch_type, staging_seq, indexer_seq
                );
                StagingTree::new(
                    batch_data.leaf_indices,
                    batch_data.leaves,
                    batch_data.nodes,
                    batch_data.node_hashes,
                    batch_data.initial_root,
                    batch_data.root_seq,
                )
            };

            match rebuild_result {
                Ok(tree) => {
                    self.staging_tree = Some(tree);
                }
                Err(e) => {
                    warn!(
                        "Failed to rebuild {} staging tree: {}. Skipping batch.",
                        batch_type, e
                    );
                    self.staging_tree = None;
                    return false;
                }
            }
        }

        true
    }

    async fn process_queue_update(&mut self, queue_work: QueueWork) -> crate::Result<usize> {
        debug!(
            "StateSupervisor processing queue update for tree {}",
            self.context.merkle_tree
        );

        // Check if we're still in the active phase before processing
        let current_slot = self.context.slot_tracker.estimated_current_slot();
        let current_phase = self
            .context
            .epoch_phases
            .get_current_epoch_state(current_slot);

        if current_phase != EpochState::Active {
            debug!(
                "Skipping queue update: not in active phase (current: {:?}, slot: {}, epoch: {})",
                current_phase, current_slot, self.context.epoch
            );
            return Ok(0);
        }

        let zkp_batch_size = self.zkp_batch_size();
        if queue_work.queue_size < zkp_batch_size {
            trace!(
                "Queue size {} below zkp_batch_size {}, skipping",
                queue_work.queue_size,
                zkp_batch_size
            );
            return Ok(0);
        }

        let phase = match queue_work.queue_type {
            QueueType::OutputStateV2 => Phase::Append,
            QueueType::InputStateV2 => Phase::Nullify,
            other => {
                warn!("Unsupported queue type for state processing: {:?}", other);
                return Ok(0);
            }
        };

        // Re-anchor to get fresh indexer data and verify consistency
        self.reanchor().await?;

        let max_batches = (queue_work.queue_size / zkp_batch_size) as usize;
        if max_batches == 0 {
            return Ok(0);
        }

        let num_workers = self.context.num_proof_workers.max(1);

        let (proof_tx, proof_rx) = mpsc::channel(num_workers * 2);
        let (job_tx, worker_handles) = spawn_proof_workers(
            num_workers,
            self.context.prover_append_url.clone(),
            self.context.prover_update_url.clone(),
            self.context.prover_api_key.clone(),
            self.context.prover_polling_interval,
            self.context.prover_max_wait_time,
            proof_tx,
        );

        let tx_sender_handle = TxSender::spawn(
            self.context.clone(),
            proof_rx,
            self.zkp_batch_size(),
            self.current_root,
        );

        self.enqueue_batches(phase, max_batches, job_tx).await?;

        for handle in worker_handles {
            match handle.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => warn!("Proof worker error: {}", e),
                Err(e) => warn!("Proof worker join error: {}", e),
            }
        }

        let tx_processed = match tx_sender_handle.await {
            Ok(res) => res?,
            Err(e) => return Err(anyhow!("Tx sender join error: {}", e)),
        };

        Ok(tx_processed)
    }

    async fn reanchor(&mut self) -> crate::Result<()> {
        let rpc = self.context.rpc_pool.get_connection().await?;
        wait_for_indexer(&*rpc).await?;

        let prev_staging_root = self.staging_tree.as_ref().map(|t| t.current_root());

        info!(
            "Reanchoring: prev staging root {:?}",
            prev_staging_root.as_ref().map(|r| format!("{:?}", &r[..4]))
        );

        // Invalidate staging tree on reanchor to get fresh indexer data
        // The staging tree will be rebuilt from indexer data in build_append_job/build_nullify_job
        self.staging_tree = None;
        let mut cache = self.context.staging_tree_cache.lock().await;
        *cache = None;

        self.seq = 0;

        Ok(())
    }

    async fn enqueue_batches(
        &mut self,
        phase: Phase,
        max_batches: usize,
        job_tx: async_channel::Sender<ProofJob>,
    ) -> crate::Result<()> {
        let zkp_batch_size = self.zkp_batch_size() as usize;
        let total_needed = max_batches.saturating_mul(zkp_batch_size);
        let fetch_len = total_needed as u64;

        let state_queue =
            fetch_batches(&self.context, None, None, fetch_len, self.zkp_batch_size()).await?;

        let Some(state_queue) = state_queue else {
            return Ok(());
        };

        let mut jobs_sent = 0usize;

        match phase {
            Phase::Append => {
                let Some(output_batch) = state_queue.output_queue.as_ref() else {
                    return Ok(());
                };
                if output_batch.leaf_indices.is_empty() {
                    return Ok(());
                }

                // Initialize state from indexer response (shared root/seq at state_queue level)
                self.current_root = state_queue.initial_root;
                self.next_index = output_batch.next_index;
                info!(
                    "Initialized from indexer: root {:?}[..4], next_index {}",
                    &self.current_root[..4],
                    self.next_index
                );

                let available = output_batch.leaf_indices.len();
                let num_slices = (available / zkp_batch_size).min(max_batches);
                for batch_idx in 0..num_slices {
                    let start = batch_idx * zkp_batch_size;
                    if let Some(job) = self
                        .build_append_job(batch_idx, &state_queue, start)
                        .await?
                    {
                        job_tx.send(job).await?;
                        jobs_sent += 1;
                    } else {
                        break;
                    }
                }
            }
            Phase::Nullify => {
                let Some(input_batch) = state_queue.input_queue.as_ref() else {
                    return Ok(());
                };
                if input_batch.leaf_indices.is_empty() {
                    return Ok(());
                }

                self.current_root = state_queue.initial_root;
                info!(
                    "Initialized from indexer: root {:?}[..4]",
                    &self.current_root[..4]
                );

                let available = input_batch.leaf_indices.len();
                let num_slices = (available / zkp_batch_size).min(max_batches);
                for batch_idx in 0..num_slices {
                    let start = batch_idx * zkp_batch_size;
                    if let Some(job) = self
                        .build_nullify_job(batch_idx, &state_queue, start)
                        .await?
                    {
                        job_tx.send(job).await?;
                        jobs_sent += 1;
                    } else {
                        break;
                    }
                }
            }
        }

        job_tx.close();

        info!("Enqueued {} jobs for proof generation", jobs_sent);
        Ok(())
    }

    async fn build_append_job(
        &mut self,
        batch_idx: usize,
        state_queue: &light_client::indexer::StateQueueDataV2,
        start: usize,
    ) -> crate::Result<Option<ProofJob>> {
        let batch = state_queue
            .output_queue
            .as_ref()
            .ok_or_else(|| anyhow!("Output queue not present in state queue"))?;

        let batch_data = StagingTreeBatchData {
            leaf_indices: &batch.leaf_indices,
            leaves: &batch.old_leaves,
            nodes: &state_queue.nodes,
            node_hashes: &state_queue.node_hashes,
            initial_root: state_queue.initial_root,
            root_seq: state_queue.root_seq,
        };

        if !self.reconcile_staging_tree(batch_idx, &batch_data, "Append") {
            return Ok(None);
        }

        let range = self.batch_range(batch.account_hashes.len(), start);
        let staging = self.staging_tree.as_mut().unwrap();
        let leaves = batch.account_hashes[range.clone()].to_vec();
        let leaf_indices = batch.leaf_indices[range].to_vec();

        let result =
            staging.process_batch_updates(&leaf_indices, &leaves, BatchType::Append, batch_idx)?;
        let new_root = result.new_root;

        let leaves_hashchain = Self::get_leaves_hashchain(&batch.leaves_hash_chains, batch_idx)?;
        let start_index =
            self.next_index
                .saturating_add((batch_idx as u64) * self.zkp_batch_size) as u32;

        let circuit_inputs =
            BatchAppendsCircuitInputs::new::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                result.into(),
                start_index,
                leaves.clone(),
                leaves_hashchain,
                self.zkp_batch_size as u32,
            )
            .map_err(|e| anyhow!("Failed to build append inputs: {}", e))?;

        self.next_index = self.next_index.saturating_add(self.zkp_batch_size);
        Ok(self
            .finish_job(new_root, ProofInput::Append(circuit_inputs))
            .await)
    }

    async fn build_nullify_job(
        &mut self,
        batch_idx: usize,
        state_queue: &light_client::indexer::StateQueueDataV2,
        start: usize,
    ) -> crate::Result<Option<ProofJob>> {
        let batch = state_queue
            .input_queue
            .as_ref()
            .ok_or_else(|| anyhow!("Input queue not present in state queue"))?;

        let batch_data = StagingTreeBatchData {
            leaf_indices: &batch.leaf_indices,
            leaves: &batch.current_leaves,
            nodes: &state_queue.nodes,
            node_hashes: &state_queue.node_hashes,
            initial_root: state_queue.initial_root,
            root_seq: state_queue.root_seq,
        };

        if !self.reconcile_staging_tree(batch_idx, &batch_data, "Nullify") {
            return Ok(None);
        }

        let range = self.batch_range(batch.account_hashes.len(), start);
        let staging = self.staging_tree.as_mut().unwrap();
        let account_hashes = batch.account_hashes[range.clone()].to_vec();
        let tx_hashes = batch.tx_hashes[range.clone()].to_vec();
        let nullifiers = batch.nullifiers[range.clone()].to_vec();
        let leaf_indices = batch.leaf_indices[range].to_vec();

        let result = staging.process_batch_updates(
            &leaf_indices,
            &nullifiers,
            BatchType::Nullify,
            batch_idx,
        )?;
        info!(
            "nullify batch {} root {:?}[..4] => {:?}[..4]",
            batch_idx,
            &result.old_root[..4],
            &result.new_root[..4]
        );

        let new_root = result.new_root;
        let leaves_hashchain = Self::get_leaves_hashchain(&batch.leaves_hash_chains, batch_idx)?;
        let path_indices: Vec<u32> = leaf_indices.iter().map(|idx| *idx as u32).collect();

        let circuit_inputs =
            BatchUpdateCircuitInputs::new::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                result.into(),
                tx_hashes,
                account_hashes,
                leaves_hashchain,
                path_indices,
                self.zkp_batch_size as u32,
            )
            .map_err(|e| anyhow!("Failed to build nullify inputs: {}", e))?;

        Ok(self
            .finish_job(new_root, ProofInput::Nullify(circuit_inputs))
            .await)
    }
}
