use anyhow::anyhow;
use forester_utils::staging_tree::{BatchType, StagingTree};
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

#[derive(Debug, Clone)]
pub struct UpdateEligibility {
    pub end_slot: u64,
}

pub struct StateSupervisor<R: Rpc> {
    context: BatchContext<R>,
    staging_tree: Option<StagingTree>,
    current_root: [u8; 32],
    next_index: u64,
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

        Ok(Self {
            context,
            staging_tree: None,
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

impl<R: Rpc> Message<UpdateEligibility> for StateSupervisor<R> {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: UpdateEligibility,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        debug!(
            "Updating eligibility end slot to {} for tree {}",
            msg.end_slot, self.context.merkle_tree
        );
        self.context
            .forester_eligibility_end_slot
            .store(msg.end_slot, std::sync::atomic::Ordering::Relaxed);
    }
}

impl<R: Rpc> StateSupervisor<R> {
    fn zkp_batch_size(&self) -> u64 {
        self.zkp_batch_size
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
    fn finish_job(&mut self, new_root: [u8; 32], inputs: ProofInput) -> Option<ProofJob> {
        self.current_root = new_root;
        self.seq += 1;
        Some(ProofJob {
            seq: self.seq - 1,
            inputs,
        })
    }

    async fn process_queue_update(&mut self, queue_work: QueueWork) -> crate::Result<usize> {
        debug!(
            "StateSupervisor processing queue update for tree {} (hint: {} items)",
            self.context.merkle_tree, queue_work.queue_size
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

        let phase = match queue_work.queue_type {
            QueueType::OutputStateV2 => Phase::Append,
            QueueType::InputStateV2 => Phase::Nullify,
            other => {
                warn!("Unsupported queue type for state processing: {:?}", other);
                return Ok(0);
            }
        };

        let num_workers = self.context.num_proof_workers.max(60);

        let (proof_tx, proof_rx) = mpsc::channel(num_workers * 2);
        let (job_tx, cancel_flag, worker_handles) =
            spawn_proof_workers(num_workers, self.context.prover_config.clone(), proof_tx);

        self.seq = 0;

        let tx_sender_handle = TxSender::spawn(
            self.context.clone(),
            proof_rx,
            self.zkp_batch_size(),
            self.current_root,
        );

        self.enqueue_batches(phase, num_workers, job_tx).await?;

        let mut had_errors = false;
        for handle in worker_handles {
            match handle.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    warn!("Proof worker error: {}", e);
                    had_errors = true;
                }
                Err(e) => {
                    warn!("Proof worker join error: {}", e);
                    had_errors = true;
                }
            }
        }

        let tx_processed = match tx_sender_handle.await {
            Ok(res) => match res {
                Ok(processed) => processed,
                Err(e) => {
                    warn!("Tx sender error, resetting staging tree: {}", e);
                    self.reset_staging_tree();
                    return Err(e);
                }
            },
            Err(e) => {
                warn!("Tx sender join error, resetting staging tree: {}", e);
                self.reset_staging_tree();
                return Err(anyhow!("Tx sender join error: {}", e));
            }
        };

        if had_errors || cancel_flag.is_cancelled() {
            warn!(
                "Errors detected (had_errors={}, cancelled={}), resetting staging tree",
                had_errors,
                cancel_flag.is_cancelled()
            );
            self.reset_staging_tree();
        }

        Ok(tx_processed)
    }

    fn reset_staging_tree(&mut self) {
        info!(
            "Resetting staging tree for tree {}",
            self.context.merkle_tree
        );
        self.staging_tree = None;
    }

    fn build_staging_tree(
        &mut self,
        leaf_indices: &[u64],
        leaves: &[[u8; 32]],
        nodes: &[u64],
        node_hashes: &[[u8; 32]],
        initial_root: [u8; 32],
        root_seq: u64,
    ) -> crate::Result<()> {
        self.staging_tree = Some(StagingTree::new(
            leaf_indices,
            leaves,
            nodes,
            node_hashes,
            initial_root,
            root_seq,
        )?);
        debug!("Built staging tree from indexer (seq={})", root_seq);
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

                self.current_root = state_queue.initial_root;
                self.next_index = output_batch.next_index;
                info!(
                    "Synced from indexer: root {:?}[..4], next_index {}",
                    &self.current_root[..4],
                    self.next_index
                );

                self.build_staging_tree(
                    &output_batch.leaf_indices,
                    &output_batch.old_leaves,
                    &state_queue.nodes,
                    &state_queue.node_hashes,
                    state_queue.initial_root,
                    state_queue.root_seq,
                )?;

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
                    "Synced from indexer: root {:?}[..4]",
                    &self.current_root[..4]
                );

                self.build_staging_tree(
                    &input_batch.leaf_indices,
                    &input_batch.current_leaves,
                    &state_queue.nodes,
                    &state_queue.node_hashes,
                    state_queue.initial_root,
                    state_queue.root_seq,
                )?;

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

        let range = self.batch_range(batch.account_hashes.len(), start);
        let leaves = batch.account_hashes[range.clone()].to_vec();
        let leaf_indices = batch.leaf_indices[range].to_vec();

        let hashchain_idx = start / self.zkp_batch_size as usize;
        let batch_seq = state_queue.root_seq + (batch_idx as u64) + 1;

        let staging = self.staging_tree.as_mut().ok_or_else(|| {
            anyhow!(
                "Staging tree not initialized for append job (batch_idx={})",
                batch_idx
            )
        })?;
        let result = staging.process_batch_updates(
            &leaf_indices,
            &leaves,
            BatchType::Append,
            batch_idx,
            batch_seq,
        )?;
        let new_root = result.new_root;

        let leaves_hashchain =
            Self::get_leaves_hashchain(&batch.leaves_hash_chains, hashchain_idx)?;
        let start_index = leaf_indices.first().copied().unwrap_or(0) as u32;

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
        Ok(self.finish_job(new_root, ProofInput::Append(circuit_inputs)))
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

        let range = self.batch_range(batch.account_hashes.len(), start);
        let account_hashes = batch.account_hashes[range.clone()].to_vec();
        let tx_hashes = batch.tx_hashes[range.clone()].to_vec();
        let nullifiers = batch.nullifiers[range.clone()].to_vec();
        let leaf_indices = batch.leaf_indices[range].to_vec();
        let hashchain_idx = start / self.zkp_batch_size as usize;
        let batch_seq = state_queue.root_seq + (batch_idx as u64) + 1;

        let staging = self.staging_tree.as_mut().ok_or_else(|| {
            anyhow!(
                "Staging tree not initialized for nullify job (batch_idx={})",
                batch_idx
            )
        })?;
        let result = staging.process_batch_updates(
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
        let leaves_hashchain =
            Self::get_leaves_hashchain(&batch.leaves_hash_chains, hashchain_idx)?;
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

        Ok(self.finish_job(new_root, ProofInput::Nullify(circuit_inputs)))
    }
}
