use anyhow::anyhow;
use forester_utils::{staging_tree::StagingTree, utils::wait_for_indexer};
use kameo::{
    actor::{ActorRef, WeakActorRef},
    error::ActorStopReason,
    message::Message,
    Actor,
};
use light_batched_merkle_tree::constants::DEFAULT_BATCH_STATE_TREE_HEIGHT;
use light_client::rpc::Rpc;
use light_compressed_account::QueueType;
use light_hasher::{Hasher, Poseidon};
use light_prover_client::proof_types::{
    batch_append::get_batch_append_inputs_v2, batch_update::get_batch_update_inputs_v2,
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

impl<R: Rpc> StateSupervisor<R> {
    fn zkp_batch_size(&self) -> u64 {
        self.zkp_batch_size
    }

    async fn process_queue_update(&mut self, queue_work: QueueWork) -> crate::Result<usize> {
        debug!(
            "ACTOR StateSupervisor processing queue update for tree {}",
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

        let (job_tx, job_rx) = mpsc::channel(8);
        let (proof_tx, proof_rx) = mpsc::channel(8);

        let worker_handles = spawn_proof_workers(
            &self.context,
            job_rx,
            proof_tx,
            self.context.prover_polling_interval,
            self.context.prover_max_wait_time,
        );
        let tx_sender_handle = TxSender::spawn(
            self.context.clone(),
            proof_rx,
            self.zkp_batch_size(),
            self.current_root,
        );

        self.enqueue_batches(phase, max_batches, job_tx).await?;

        let mut worker_errors = Vec::new();
        for handle in worker_handles {
            match handle.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => worker_errors.push(e),
                Err(join_err) => {
                    worker_errors.push(anyhow!("Proof worker task join error: {}", join_err))
                }
            }
        }

        if let Some(err) = worker_errors.into_iter().next() {
            return Err(err);
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
        job_tx: mpsc::Sender<ProofJob>,
    ) -> crate::Result<()> {
        let zkp_batch_size = self.zkp_batch_size() as usize;
        let total_needed = max_batches.saturating_mul(zkp_batch_size);
        let fetch_len = total_needed as u64;

        let (output_batch, input_batch) =
            fetch_batches(&self.context, None, None, fetch_len, self.zkp_batch_size()).await?;

        match phase {
            Phase::Append => {
                let Some(batch) = output_batch else {
                    return Ok(());
                };
                if batch.leaf_indices.is_empty() {
                    return Ok(());
                }

                // Initialize state from indexer response
                self.current_root = batch.initial_root;
                self.next_index = batch.next_index;
                info!(
                    "Initialized from indexer: root {:?}[..4], next_index {}",
                    &self.current_root[..4],
                    self.next_index
                );

                let available = batch.leaf_indices.len();
                let num_slices = (available / zkp_batch_size).min(max_batches);
                for batch_idx in 0..num_slices {
                    let start = batch_idx * zkp_batch_size;
                    if let Some(job) = self.build_append_job(batch_idx, &batch, start).await? {
                        job_tx.send(job).await?;
                    } else {
                        break;
                    }
                }
            }
            Phase::Nullify => {
                let Some(batch) = input_batch else {
                    return Ok(());
                };
                if batch.leaf_indices.is_empty() {
                    return Ok(());
                }

                self.current_root = batch.initial_root;
                info!(
                    "Initialized from indexer: root {:?}[..4]",
                    &self.current_root[..4]
                );

                let available = batch.leaf_indices.len();
                let num_slices = (available / zkp_batch_size).min(max_batches);
                for batch_idx in 0..num_slices {
                    let start = batch_idx * zkp_batch_size;
                    if let Some(job) = self.build_nullify_job(batch_idx, &batch, start).await? {
                        job_tx.send(job).await?;
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    async fn build_append_job(
        &mut self,
        batch_idx: usize,
        batch: &light_client::indexer::OutputQueueDataV2,
        start: usize,
    ) -> crate::Result<Option<ProofJob>> {
        // Initialize or reset staging tree from indexer data
        if self.staging_tree.is_none() {
            match StagingTree::from_v2_output_queue(
                &batch.leaf_indices,
                &batch.old_leaves,
                &batch.nodes,
                &batch.node_hashes,
                batch.initial_root,
            ) {
                Ok(tree) => {
                    self.staging_tree = Some(tree);
                }
                Err(e) => {
                    warn!(
                        "Failed to initialize staging tree from indexer data: {}. Retrying later.",
                        e
                    );
                    return Ok(None);
                }
            }
        }

        let zkp_batch_size = self.zkp_batch_size() as usize;

        // Validate staging tree root matches indexer data for first batch
        if batch_idx == 0
            && self
                .staging_tree
                .as_ref()
                .map(|t| t.current_root() != batch.initial_root)
                .unwrap_or(true)
        {
            match StagingTree::from_v2_output_queue(
                &batch.leaf_indices,
                &batch.old_leaves,
                &batch.nodes,
                &batch.node_hashes,
                batch.initial_root,
            ) {
                Ok(tree) => {
                    self.staging_tree = Some(tree);
                }
                Err(_) => {
                    self.staging_tree = None;
                    return Ok(None);
                }
            }
        }

        let staging = self.staging_tree.as_mut().unwrap();
        let end = (start + zkp_batch_size).min(batch.account_hashes.len());
        let leaves = batch.account_hashes[start..end].to_vec();
        let leaf_indices = batch.leaf_indices[start..end].to_vec();

        let (old_leaves, merkle_proofs, old_root, new_root) =
            staging.process_batch_updates(&leaf_indices, &leaves, "APPEND", batch_idx)?;

        let leaves_hashchain = batch
            .leaves_hash_chains
            .get(batch_idx)
            .copied()
            .ok_or_else(|| {
                anyhow!(
                    "Missing leaves_hash_chain for batch {} (available: {})",
                    batch_idx,
                    batch.leaves_hash_chains.len()
                )
            })?;

        let start_index = self
            .next_index
            .saturating_add((batch_idx as u64) * self.zkp_batch_size())
            as u32;

        let circuit_inputs =
            get_batch_append_inputs_v2::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                old_root,
                start_index,
                leaves.clone(),
                leaves_hashchain,
                old_leaves,
                merkle_proofs,
                self.zkp_batch_size() as u32,
                new_root,
            )
            .map_err(|e| anyhow!("Failed to build append inputs: {}", e))?;

        self.current_root = new_root;
        self.next_index = self.next_index.saturating_add(self.zkp_batch_size());
        self.seq += 1;

        {
            let mut cache = self.context.staging_tree_cache.lock().await;
            *cache = self.staging_tree.clone();
        }

        Ok(Some(ProofJob {
            seq: self.seq - 1,
            inputs: ProofInput::Append(circuit_inputs),
        }))
    }

    async fn build_nullify_job(
        &mut self,
        batch_idx: usize,
        batch: &light_client::indexer::InputQueueDataV2,
        start: usize,
    ) -> crate::Result<Option<ProofJob>> {
        if self.staging_tree.is_none() {
            match StagingTree::from_v2_input_queue(
                &batch.leaf_indices,
                &batch.current_leaves,
                &batch.nodes,
                &batch.node_hashes,
                batch.initial_root,
            ) {
                Ok(tree) => {
                    self.staging_tree = Some(tree);
                }
                Err(e) => {
                    warn!(
                        "Failed to initialize nullify staging tree from indexer data: {}. Retrying later.",
                        e
                    );
                    return Ok(None);
                }
            }
        }

        let zkp_batch_size = self.zkp_batch_size() as usize;

        if batch_idx == 0
            && self
                .staging_tree
                .as_ref()
                .map(|t| t.current_root() != batch.initial_root)
                .unwrap_or(true)
        {
            let prev = self
                .staging_tree
                .as_ref()
                .map(|t| t.current_root())
                .unwrap_or([0u8; 32]);
            warn!(
                "Staging root {:?}[..4] != batch initial {:?}[..4], resetting from indexer snapshot",
                &prev[..4],
                &batch.initial_root[..4]
            );
            match StagingTree::from_v2_input_queue(
                &batch.leaf_indices,
                &batch.current_leaves,
                &batch.nodes,
                &batch.node_hashes,
                batch.initial_root,
            ) {
                Ok(tree) => {
                    self.staging_tree = Some(tree);
                }
                Err(e) => {
                    warn!(
                        "Failed to reset nullify staging from indexer snapshot: {}. Skipping batch.",
                        e
                    );
                    self.staging_tree = None;
                    return Ok(None);
                }
            }
        }

        let staging = self.staging_tree.as_mut().unwrap();
        let end = (start + zkp_batch_size).min(batch.account_hashes.len());
        let account_hashes = batch.account_hashes[start..end].to_vec();
        let tx_hashes = batch.tx_hashes[start..end].to_vec();
        let leaf_indices = batch.leaf_indices[start..end].to_vec();

        let mut nullifiers = Vec::with_capacity(zkp_batch_size);
        for (idx, account_hash) in account_hashes.iter().enumerate() {
            let mut leaf_bytes = [0u8; 32];
            leaf_bytes[24..].copy_from_slice(&leaf_indices[idx].to_be_bytes());
            let nullifier =
                Poseidon::hashv(&[account_hash.as_slice(), &leaf_bytes, &tx_hashes[idx]])
                    .map_err(|e| anyhow!("Failed to compute nullifier {}: {}", idx, e))?;
            nullifiers.push(nullifier);
        }

        let (old_leaves, merkle_proofs, old_root, new_root) =
            staging.process_batch_updates(&leaf_indices, &nullifiers, "NULLIFY", batch_idx)?;
        info!(
            "nullify batch {} root {:?}[..4] => {:?}[..4]",
            batch_idx,
            &old_root[..4],
            &new_root[..4]
        );

        let leaves_hashchain = batch
            .leaves_hash_chains
            .get(batch_idx)
            .copied()
            .ok_or_else(|| {
                anyhow!(
                    "Missing leaves_hash_chain for batch {} (available: {})",
                    batch_idx,
                    batch.leaves_hash_chains.len()
                )
            })?;

        let path_indices: Vec<u32> = leaf_indices.iter().map(|idx| *idx as u32).collect();

        let circuit_inputs =
            get_batch_update_inputs_v2::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                old_root,
                tx_hashes,
                account_hashes,
                leaves_hashchain,
                old_leaves,
                merkle_proofs,
                path_indices,
                self.zkp_batch_size() as u32,
                new_root,
            )
            .map_err(|e| anyhow!("Failed to build nullify inputs: {}", e))?;

        self.current_root = new_root;
        self.seq += 1;

        {
            let mut cache = self.context.staging_tree_cache.lock().await;
            *cache = self.staging_tree.clone();
        }

        Ok(Some(ProofJob {
            seq: self.seq - 1,
            inputs: ProofInput::Nullify(circuit_inputs),
        }))
    }
}
