use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use anyhow::anyhow;
use borsh::BorshSerialize;
use forester_utils::instructions::state::BatchInstruction;
use forester_utils::staging_tree::StagingTree;
use forester_utils::{ParsedMerkleTreeData, ParsedQueueData};
use kameo::{
    actor::{ActorRef, WeakActorRef},
    error::ActorStopReason,
    message::Message,
    Actor,
};
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT,
    merkle_tree::{
        BatchedMerkleTreeAccount, InstructionDataBatchAppendInputs,
        InstructionDataBatchNullifyInputs,
    },
    queue::BatchedQueueAccount,
};
use light_client::{
    indexer::{Indexer, QueueElementsV2Options},
    rpc::Rpc,
};
use light_compressed_account::QueueType;
use light_hasher::{hash_chain::create_hash_chain_from_slice, Hasher, Poseidon};
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::{
        batch_append::{get_batch_append_inputs_v2, BatchAppendsCircuitInputs},
        batch_update::{get_batch_update_inputs_v2, BatchUpdateCircuitInputs},
    },
};
use light_registry::{
    account_compression_cpi::sdk::{
        create_batch_append_instruction, create_batch_nullify_instruction,
    },
    protocol_config::state::EpochState,
};
use solana_sdk::{account::Account, signer::Signer};
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::{debug, info, trace, warn};

use super::common::{send_transaction_batch, BatchContext};
use crate::{errors::ForesterError, Result};
use forester_utils::utils::wait_for_indexer;

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

#[derive(Debug)]
struct ProofJob {
    seq: u64,
    inputs: ProofInput,
}

#[derive(Debug)]
enum ProofInput {
    Append(BatchAppendsCircuitInputs),
    Nullify(BatchUpdateCircuitInputs),
}

#[derive(Debug)]
struct ProofResult {
    seq: u64,
    instruction: BatchInstruction,
}

// Message types for StateSupervisor actor
#[derive(Debug, Clone)]
pub struct ProcessQueueUpdate {
    pub queue_work: QueueWork,
}

// Actor implementation
pub struct StateSupervisor<R: Rpc> {
    context: BatchContext<R>,
    tree_state: ParsedMerkleTreeData,
    output_queue_state: Option<ParsedQueueData>,
    staging_tree: Option<StagingTree>,
    append_hash_chains: VecDeque<[u8; 32]>,
    nullify_hash_chains: VecDeque<[u8; 32]>,
    seq: u64,
}

// Actor trait implementation
impl<R: Rpc> Actor for StateSupervisor<R> {
    type Args = BatchContext<R>;
    type Error = anyhow::Error;

    async fn on_start(
        context: Self::Args,
        _actor_ref: ActorRef<Self>,
    ) -> std::result::Result<Self, Self::Error> {
        info!(
            "StateSupervisor actor starting for tree {}",
            context.merkle_tree
        );

        let tree_state = fetch_state_tree(&context).await?;
        let output_queue_state = fetch_output_queue(&context).await?;

        let staging_tree = {
            let mut cache = context.staging_tree_cache.lock().await;
            if let Some(cached) = cache.as_ref() {
                if cached.current_root() == tree_state.current_root {
                    Some(cached.clone())
                } else {
                    *cache = None;
                    None
                }
            } else {
                None
            }
        };

        Ok(Self {
            context,
            staging_tree,
            append_hash_chains: output_queue_state
                .as_ref()
                .map(|q| VecDeque::from(q.leaves_hash_chains.clone()))
                .unwrap_or_default(),
            nullify_hash_chains: VecDeque::from(tree_state.leaves_hash_chains.clone()),
            tree_state,
            output_queue_state,
            seq: 0,
        })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> std::result::Result<(), Self::Error> {
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

// Message handler for ProcessQueueUpdate
impl<R: Rpc> Message<ProcessQueueUpdate> for StateSupervisor<R> {
    type Reply = Result<usize>;

    async fn handle(
        &mut self,
        msg: ProcessQueueUpdate,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.process_queue_update(msg.queue_work).await
    }
}

impl<R: Rpc> StateSupervisor<R> {
    fn zkp_batch_size(&self) -> u16 {
        self.output_queue_state
            .as_ref()
            .map(|q| q.zkp_batch_size)
            .unwrap_or(self.tree_state.zkp_batch_size)
    }

    async fn process_queue_update(&mut self, queue_work: QueueWork) -> Result<usize> {
        debug!(
            "ACTOR StateSupervisor processing queue update for tree {}",
            self.context.merkle_tree
        );

        // Check if we're still in the active phase before processing
        let current_slot = self.context.slot_tracker.estimated_current_slot();
        let current_phase = self.context.epoch_phases.get_current_epoch_state(current_slot);

        if current_phase != EpochState::Active {
            debug!(
                "ACTOR Skipping queue update: not in active phase (current: {:?}, slot: {}, epoch: {})",
                current_phase,
                current_slot,
                self.context.epoch
            );
            return Ok(0);
        }

        let zkp_batch_size = self.zkp_batch_size() as u64;
        if queue_work.queue_size < zkp_batch_size {
            trace!(
                "ACTOR Queue size {} below zkp_batch_size {}, skipping",
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
            self.tree_state.current_root,
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

    async fn reanchor(&mut self) -> Result<()> {
        let rpc = self.context.rpc_pool.get_connection().await?;
        wait_for_indexer(&*rpc).await?;

        let tree_state = fetch_state_tree(&self.context).await?;
        let output_queue_state = fetch_output_queue(&self.context).await?;

        let onchain_root = tree_state.current_root;
        let prev_staging_root = self.staging_tree.as_ref().map(|t| t.current_root());

        info!(
            "ACTOR Reanchoring: on-chain root {:?}[..4], prev staging root {:?}",
            &onchain_root[..4],
            prev_staging_root.as_ref().map(|r| format!("{:?}", &r[..4]))
        );

        // Always invalidate staging tree and cache on reanchor to get fresh indexer data
        // This helps diagnose if indexer is returning incorrect data
        self.staging_tree = None;
        let mut cache = self.context.staging_tree_cache.lock().await;
        *cache = None;

        self.tree_state = tree_state;
        self.output_queue_state = output_queue_state;
        self.append_hash_chains = self
            .output_queue_state
            .as_ref()
            .map(|q| VecDeque::from(q.leaves_hash_chains.clone()))
            .unwrap_or_default();
        debug!(
            "ACTOR Reanchored with {} append hash chains, first: {:?}[..4]",
            self.append_hash_chains.len(),
            self.append_hash_chains.get(0).map(|h| &h[..4])
        );
        self.nullify_hash_chains = VecDeque::from(self.tree_state.leaves_hash_chains.clone());
        self.seq = 0;

        Ok(())
    }

    async fn enqueue_batches(
        &mut self,
        phase: Phase,
        max_batches: usize,
        job_tx: mpsc::Sender<ProofJob>,
    ) -> Result<()> {
        // Determine how many elements to fetch in a single indexer call
        let zkp_batch_size = self.zkp_batch_size() as usize;
        let total_needed = max_batches.saturating_mul(zkp_batch_size);
        let fetch_len: u16 = total_needed
            .min(u16::MAX as usize)
            .try_into()
            .unwrap_or(u16::MAX);

        let (output_batch, input_batch) = fetch_batches(
            &self.context,
            None,
            None,
            fetch_len,
            self.zkp_batch_size(),
        )
        .await?;

        match phase {
            Phase::Append => {
                let Some(batch) = output_batch else { return Ok(()); };
                if batch.leaf_indices.is_empty() { return Ok(()); }

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
                let Some(batch) = input_batch else { return Ok(()); };
                if batch.leaf_indices.is_empty() { return Ok(()); }

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
    ) -> Result<Option<ProofJob>> {
        // Verify indexer snapshot only for the first slice to avoid unnecessary resets
        if batch_idx == 0 {
            if let Some(onchain_root) = current_onchain_root(&self.context).await? {
                if onchain_root != batch.initial_root {
                    warn!(
                        "ACTOR INDEXER ISSUE: on-chain root {:?}[..4] != batch.initial_root {:?}[..4]. Indexer returned incorrect data!",
                        &onchain_root[..4],
                        &batch.initial_root[..4]
                    );
                    self.staging_tree = None;
                    let mut cache = self.context.staging_tree_cache.lock().await;
                    *cache = None;
                    return Ok(None);
                }
            }
        }

        if self.staging_tree.is_none() {
            info!(
                "ACTOR Rebuilding staging tree from indexer batch data (initial_root {:?}[..4])",
                &batch.initial_root[..4]
            );
            debug!(
                "ACTOR OutputQueueDataV2: {} account_hashes, {} leaf_indices, {} old_leaves, first 3 account_hashes: {:?}, {:?}, {:?}",
                batch.account_hashes.len(),
                batch.leaf_indices.len(),
                batch.old_leaves.len(),
                if batch.account_hashes.len() > 0 { &batch.account_hashes[0][..4] } else { &[0u8; 4] },
                if batch.account_hashes.len() > 1 { &batch.account_hashes[1][..4] } else { &[0u8; 4] },
                if batch.account_hashes.len() > 2 { &batch.account_hashes[2][..4] } else { &[0u8; 4] }
            );
            match StagingTree::from_v2_output_queue(
                &batch.leaf_indices,
                &batch.old_leaves,
                &batch.nodes,
                &batch.node_hashes,
                batch.initial_root,
            ) {
                Ok(tree) => {
                    info!(
                        "ACTOR Staging tree rebuilt, root: {:?}[..4]",
                        &tree.current_root()[..4]
                    );
                    self.staging_tree = Some(tree);
                }
                Err(e) => {
                    warn!(
                        "ACTOR Failed to initialize staging tree from indexer data: {}. Dropping cache and retrying later.",
                        e
                    );
                    self.staging_tree = None;
                    return Ok(None);
                }
            }
        }

        let zkp_batch_size = self.zkp_batch_size() as usize;
        // Only allow staging reset from snapshot on the very first slice
        if batch_idx == 0 {
            if self
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
                    "ACTOR Staging root {:?}[..4] != batch initial {:?}[..4], resetting staging from batch snapshot",
                    &prev[..4],
                    &batch.initial_root[..4]
                );
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
                            "ACTOR Failed  to reset staging from batch snapshot: {}. Skipping batch.",
                            e
                        );
                        self.staging_tree = None;
                        return Ok(None);
                    }
                }
            }
        }

        let staging = self.staging_tree.as_mut().unwrap();
        let end = (start + zkp_batch_size).min(batch.account_hashes.len());
        let leaves = batch.account_hashes[start..end].to_vec();
        let leaf_indices = batch.leaf_indices[start..end].to_vec();

        let (old_leaves, merkle_proofs, old_root, new_root) =
            staging.process_batch_updates(&leaf_indices, &leaves, "APPEND", batch_idx)?;
        info!(
            "ACTOR APPEND batch {} root {:?}[..4] => {:?}[..4]",
            batch_idx,
            &old_root[..4],
            &new_root[..4]
        );

        if let Some(expected_hash) = self.append_hash_chains.get(batch_idx).copied() {
            debug!(
                "ACTOR APPEND hashchain check batch {}: {} leaves from Photon, first 3: {:?}, {:?}, {:?}",
                batch_idx,
                leaves.len(),
                if leaves.len() > 0 { &leaves[0][..4] } else { &[0u8; 4] },
                if leaves.len() > 1 { &leaves[1][..4] } else { &[0u8; 4] },
                if leaves.len() > 2 { &leaves[2][..4] } else { &[0u8; 4] }
            );
            let computed_hash = create_hash_chain_from_slice(&leaves)
                .map_err(|e| anyhow!("Failed to recompute append hashchain: {}", e))?;
            if expected_hash != computed_hash {
                warn!(
                    "ACTOR Append hashchain MISMATCH at batch {}:\n  expected: {:?}\n  computed: {:?}\n  leaves: {:?}",
                    batch_idx,
                    expected_hash,
                    computed_hash,
                    leaves.iter().take(3).collect::<Vec<_>>()
                );
                return Err(anyhow!(
                    "ACTOR Append hashchain mismatch at batch {} expected {:?}[..4] got {:?}[..4]",
                    batch_idx,
                    &expected_hash[..4],
                    &computed_hash[..4]
                ));
            } else {
                debug!("ACTOR APPEND hashchain matches for batch {}", batch_idx);
            }
        }

        let start_index = self
            .tree_state
            .next_index
            .saturating_add((batch_idx as u64) * self.zkp_batch_size() as u64)
            as u32;

        // DEBUG: Dump data for unit test comparison (only for batch 0)
        if batch_idx == 0 && std::env::var("DUMP_PHOTON_DATA").is_ok() {
            let dump_data = serde_json::json!({
                "old_root": old_root,
                "start_index": start_index,
                "leaves": leaves,
                "leaves_hashchain": self.append_hash_chains.get(batch_idx).unwrap_or(&[0u8; 32]),
                "old_leaves": old_leaves,
                "merkle_proofs": merkle_proofs,
                "batch_size": self.zkp_batch_size(),
                "leaf_indices": leaf_indices,
                "new_root": new_root,
                "operation": "append"
            });
            if let Err(e) = std::fs::write("/tmp/photon_append_batch0.json", serde_json::to_string_pretty(&dump_data).unwrap()) {
                warn!("Failed to dump Photon data: {}", e);
            } else {
                info!("DEBUG: Dumped Photon APPEND data to /tmp/photon_append_batch0.json");
            }
        }

        let circuit_inputs =
            get_batch_append_inputs_v2::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                old_root,
                start_index,
                leaves.clone(),
                *self.append_hash_chains.get(batch_idx).unwrap_or(&[0u8; 32]),
                old_leaves,
                merkle_proofs,
                self.zkp_batch_size() as u32,
                new_root,
            )
            .map_err(|e| anyhow!("ACTOR Failed to build append inputs: {}", e))?;

        self.tree_state.current_root = new_root;
        self.tree_state.next_index = self
            .tree_state
            .next_index
            .saturating_add(self.zkp_batch_size() as u64);
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
    ) -> Result<Option<ProofJob>> {
        // Verify indexer snapshot only for the first slice
        if batch_idx == 0 {
            if let Some(onchain_root) = current_onchain_root(&self.context).await? {
                if onchain_root != batch.initial_root {
                    warn!(
                        "ACTOR INDEXER ISSUE: on-chain root {:?}[..4] != batch.initial_root {:?}[..4]. Indexer returned incorrect data!",
                        &onchain_root[..4],
                        &batch.initial_root[..4]
                    );
                    self.staging_tree = None;
                    let mut cache = self.context.staging_tree_cache.lock().await;
                    *cache = None;
                    return Ok(None);
                }
            }
        }

        if self.staging_tree.is_none() {
            info!(
                "ACTOR Rebuilding staging tree from indexer input queue (initial_root {:?}[..4])",
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
                    info!(
                        "ACTOR Staging tree rebuilt from input queue, root: {:?}[..4]",
                        &tree.current_root()[..4]
                    );
                    self.staging_tree = Some(tree);
                }
                Err(e) => {
                    warn!(
                        "ACTOR Failed to initialize nullify staging tree from indexer data: {}. Dropping cache and waiting for indexer sync.",
                        e
                    );
                    self.staging_tree = None;
                    let rpc = self.context.rpc_pool.get_connection().await?;
                    let _ = wait_for_indexer(&*rpc).await;
                    return Ok(None);
                }
            }
        }

        let zkp_batch_size = self.zkp_batch_size() as usize;
        // Only allow reset for the first slice
        if batch_idx == 0 {
            if self
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
                    "ACTOR Staging root {:?}[..4] != batch initial {:?}[..4], resetting staging from batch snapshot",
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
                            "ACTOR Failed to reset nullify staging from batch snapshot: {}. Skipping batch.",
                            e
                        );
                        self.staging_tree = None;
                        return Ok(None);
                    }
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
                    .map_err(|e| anyhow!("ACTOR Failed to compute nullifier {}: {}", idx, e))?;
            nullifiers.push(nullifier);
        }

        let (old_leaves, merkle_proofs, old_root, new_root) =
            staging.process_batch_updates(&leaf_indices, &nullifiers, "NULLIFY", batch_idx)?;
        info!(
            "ACTOR NULLIFY batch {} root {:?}[..4] => {:?}[..4]",
            batch_idx,
            &old_root[..4],
            &new_root[..4]
        );

        if let Some(expected_hash) = self.nullify_hash_chains.get(batch_idx).copied() {
            let computed_hash =
                create_hash_chain_from_slice(&nullifiers).map_err(|e| anyhow!(e.to_string()))?;
            if expected_hash != computed_hash {
                return Err(anyhow!(
                    "ACTOR Nullify hashchain mismatch at batch {} expected {:?}[..4] got {:?}[..4]",
                    batch_idx,
                    &expected_hash[..4],
                    &computed_hash[..4]
                ));
            }
            else {
                debug!("ACTOR NULLIFY hashchain matches for batch {}", batch_idx);
            }
        }

        let path_indices: Vec<u32> = leaf_indices.iter().map(|idx| *idx as u32).collect();

        // DEBUG: Dump data for unit test comparison (only for batch 0)
        if batch_idx == 0 && std::env::var("DUMP_PHOTON_DATA").is_ok() {
            let dump_data = serde_json::json!({
                "old_root": old_root,
                "tx_hashes": tx_hashes,
                "account_hashes": account_hashes,
                "leaves_hashchain": self.nullify_hash_chains.get(batch_idx).unwrap_or(&[0u8; 32]),
                "old_leaves": old_leaves,
                "merkle_proofs": merkle_proofs,
                "path_indices": path_indices,
                "batch_size": self.zkp_batch_size(),
                "leaf_indices": leaf_indices,
                "new_root": new_root,
                "operation": "nullify"
            });
            if let Err(e) = std::fs::write("/tmp/photon_nullify_batch0.json", serde_json::to_string_pretty(&dump_data).unwrap()) {
                warn!("Failed to dump Photon data: {}", e);
            } else {
                info!("DEBUG: Dumped Photon NULLIFY data to /tmp/photon_nullify_batch0.json");
            }
        }

        let circuit_inputs =
            get_batch_update_inputs_v2::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                old_root,
                tx_hashes,
                account_hashes,
                *self
                    .nullify_hash_chains
                    .get(batch_idx)
                    .unwrap_or(&[0u8; 32]),
                old_leaves,
                merkle_proofs,
                path_indices,
                self.zkp_batch_size() as u32,
                new_root,
            )
            .map_err(|e| anyhow!("ACTOR Failed to build nullify inputs: {}", e))?;

        self.tree_state.current_root = new_root;
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

fn spawn_proof_workers<R: Rpc>(
    context: &BatchContext<R>,
    mut job_rx: mpsc::Receiver<ProofJob>,
    proof_tx: mpsc::Sender<ProofResult>,
    polling_interval: Duration,
    max_wait_time: Duration,
) -> Vec<JoinHandle<Result<()>>> {
    let append_client = ProofClient::with_config(
        context.prover_append_url.clone(),
        polling_interval,
        max_wait_time,
        context.prover_api_key.clone(),
    );
    let nullify_client = ProofClient::with_config(
        context.prover_update_url.clone(),
        polling_interval,
        max_wait_time,
        context.prover_api_key.clone(),
    );
    let proof_tx_clone = proof_tx.clone();

    let handle = tokio::spawn(async move {
        while let Some(job) = job_rx.recv().await {
            match job.inputs {
                ProofInput::Append(inputs) => {
                    let (proof, new_root) = append_client
                        .generate_batch_append_proof(inputs)
                        .await
                        .map_err(|e| anyhow!("ACTOR Append proof generation failed: {}", e))?;
                    let instruction = InstructionDataBatchAppendInputs {
                        new_root,
                        compressed_proof: light_compressed_account::instruction_data::compressed_proof::CompressedProof { a: proof.a, b: proof.b, c: proof.c },
                    };
                    proof_tx_clone
                        .send(ProofResult {
                            seq: job.seq,
                            instruction: BatchInstruction::Append(vec![instruction]),
                        })
                        .await?;
                }
                ProofInput::Nullify(inputs) => {
                    let (proof, new_root) = nullify_client
                        .generate_batch_update_proof(inputs)
                        .await
                        .map_err(|e| anyhow!("ACTOR Nullify proof generation failed: {}", e))?;
                    let instruction = InstructionDataBatchNullifyInputs {
                        new_root,
                        compressed_proof: light_compressed_account::instruction_data::compressed_proof::CompressedProof { a: proof.a, b: proof.b, c: proof.c },
                    };
                    proof_tx_clone
                        .send(ProofResult {
                            seq: job.seq,
                            instruction: BatchInstruction::Nullify(vec![instruction]),
                        })
                        .await?;
                }
            }
        }
        Ok(())
    });

    vec![handle]
}

struct TxSender<R: Rpc> {
    context: BatchContext<R>,
    expected_seq: u64,
    buffer: BTreeMap<u64, BatchInstruction>,
    zkp_batch_size: u16,
    last_seen_root: [u8; 32],
}

impl<R: Rpc> TxSender<R> {
    fn spawn(
        context: BatchContext<R>,
        proof_rx: mpsc::Receiver<ProofResult>,
        zkp_batch_size: u16,
        last_seen_root: [u8; 32],
    ) -> JoinHandle<Result<usize>> {
        let sender = Self {
            context,
            expected_seq: 0,
            buffer: BTreeMap::new(),
            zkp_batch_size,
            last_seen_root,
        };

        tokio::spawn(async move { sender.run(proof_rx).await })
    }

    async fn run(mut self, mut proof_rx: mpsc::Receiver<ProofResult>) -> Result<usize> {
        let mut processed = 0usize;

        while let Some(result) = proof_rx.recv().await {
            self.buffer.insert(result.seq, result.instruction);

            while let Some(instr) = self.buffer.remove(&self.expected_seq) {
                let (instructions, expected_root) = match &instr {
                    BatchInstruction::Append(proofs) => {
                        let ix = proofs
                            .iter()
                            .map(|data| {
                                create_batch_append_instruction(
                                    self.context.authority.pubkey(),
                                    self.context.derivation,
                                    self.context.merkle_tree,
                                    self.context.output_queue,
                                    self.context.epoch,
                                    data.try_to_vec().unwrap(),
                                )
                            })
                            .collect::<Vec<_>>();
                        (ix, proofs.last().map(|p| p.new_root))
                    }
                    BatchInstruction::Nullify(proofs) => {
                        let ix = proofs
                            .iter()
                            .map(|data| {
                                create_batch_nullify_instruction(
                                    self.context.authority.pubkey(),
                                    self.context.derivation,
                                    self.context.merkle_tree,
                                    self.context.epoch,
                                    data.try_to_vec().unwrap(),
                                )
                            })
                            .collect::<Vec<_>>();
                        (ix, proofs.last().map(|p| p.new_root))
                    }
                };

                match send_transaction_batch(&self.context, instructions).await {
                    Ok(sig) => {
                        if let Some(root) = expected_root {
                            self.last_seen_root = root;
                        }
                        processed += self.zkp_batch_size as usize;
                        self.expected_seq += 1;
                        info!(
                            "ACTOR tx sent {} root {:?} seq {} epoch {}",
                            sig, self.last_seen_root, self.expected_seq, self.context.epoch
                        );
                    }
                    Err(e) => {
                        info!("ACTOR tx error {} epoch {}", e, self.context.epoch);
                        if let Some(ForesterError::NotInActivePhase) =
                            e.downcast_ref::<ForesterError>()
                        {
                            warn!("Active phase ended while sending tx, stopping sender loop");
                            return Ok(processed);
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
        }

        Ok(processed)
    }
}

async fn fetch_state_tree<R: Rpc>(context: &BatchContext<R>) -> Result<ParsedMerkleTreeData> {
    let rpc = context.rpc_pool.get_connection().await?;
    let mut account = rpc
        .get_account(context.merkle_tree)
        .await?
        .ok_or_else(|| anyhow!("Merkle tree account missing"))?;

    parse_state_tree(&mut account, context.merkle_tree)
}

async fn fetch_output_queue<R: Rpc>(context: &BatchContext<R>) -> Result<Option<ParsedQueueData>> {
    let rpc = context.rpc_pool.get_connection().await?;
    let Some(mut account) = rpc.get_account(context.output_queue).await? else {
        return Ok(None);
    };

    parse_output_queue(&mut account)
}

fn parse_state_tree(
    account: &mut Account,
    merkle_tree: solana_sdk::pubkey::Pubkey,
) -> Result<ParsedMerkleTreeData> {
    let tree = BatchedMerkleTreeAccount::state_from_bytes(
        account.data.as_mut_slice(),
        &merkle_tree.into(),
    )?;

    let batch_index = tree.queue_batches.pending_batch_index;
    let batch = tree
        .queue_batches
        .batches
        .get(batch_index as usize)
        .ok_or_else(|| anyhow!("Batch not found"))?;

    let num_inserted_zkps = batch.get_num_inserted_zkps();
    let current_zkp_batch_index = batch.get_current_zkp_batch_index();

    let mut leaves_hash_chains = Vec::new();
    for i in num_inserted_zkps..current_zkp_batch_index {
        leaves_hash_chains.push(tree.hash_chain_stores[batch_index as usize][i as usize]);
    }

    let onchain_root = *tree.root_history.last().unwrap();

    Ok(ParsedMerkleTreeData {
        next_index: tree.next_index,
        current_root: onchain_root,
        root_history: tree.root_history.to_vec(),
        zkp_batch_size: batch.zkp_batch_size as u16,
        pending_batch_index: batch_index as u32,
        num_inserted_zkps,
        current_zkp_batch_index,
        batch_start_index: batch.start_index,
        leaves_hash_chains,
    })
}

fn parse_output_queue(account: &mut Account) -> Result<Option<ParsedQueueData>> {
    let output_queue = BatchedQueueAccount::output_from_bytes(account.data.as_mut_slice())?;

    let batch_index = output_queue.batch_metadata.pending_batch_index;
    let batch = output_queue
        .batch_metadata
        .batches
        .get(batch_index as usize)
        .ok_or_else(|| anyhow!("Batch not found"))?;

    let num_inserted_zkps = batch.get_num_inserted_zkps();
    let current_zkp_batch_index = batch.get_current_zkp_batch_index();

    let mut leaves_hash_chains = Vec::new();
    for i in num_inserted_zkps..current_zkp_batch_index {
        leaves_hash_chains.push(output_queue.hash_chain_stores[batch_index as usize][i as usize]);
    }

    let parsed = ParsedQueueData {
        zkp_batch_size: output_queue.batch_metadata.zkp_batch_size as u16,
        pending_batch_index: batch_index as u32,
        num_inserted_zkps,
        current_zkp_batch_index,
        leaves_hash_chains,
    };

    Ok(Some(parsed))
}

async fn current_onchain_root<R: Rpc>(context: &BatchContext<R>) -> Result<Option<[u8; 32]>> {
    let rpc = context.rpc_pool.get_connection().await?;
    let Some(mut account) = rpc.get_account(context.merkle_tree).await? else {
        return Ok(None);
    };
    let tree = BatchedMerkleTreeAccount::state_from_bytes(
        account.data.as_mut_slice(),
        &context.merkle_tree.into(),
    )?;
    Ok(tree.get_root())
}

async fn fetch_batches<R: Rpc>(
    context: &BatchContext<R>,
    output_start_index: Option<u64>,
    input_start_index: Option<u64>,
    fetch_len: u16,
    zkp_batch_size: u16,
) -> Result<(
    Option<light_client::indexer::OutputQueueDataV2>,
    Option<light_client::indexer::InputQueueDataV2>,
)> {
    let mut rpc = context.rpc_pool.get_connection().await?;
    let indexer = rpc.indexer_mut()?;
    let options = QueueElementsV2Options::default()
        .with_output_queue(output_start_index, Some(fetch_len))
        .with_output_queue_batch_size(Some(zkp_batch_size))
        .with_input_queue(input_start_index, Some(fetch_len))
        .with_input_queue_batch_size(Some(zkp_batch_size));

    let res = indexer
        .get_queue_elements_v2(context.merkle_tree.to_bytes(), options, None)
        .await?;

    Ok((res.value.output_queue, res.value.input_queue))
}
