use anyhow::anyhow;
use forester_utils::address_staging_tree::AddressStagingTree;
use kameo::{
    actor::{ActorRef, WeakActorRef},
    error::ActorStopReason,
    message::Message,
    Actor,
};
use light_client::rpc::Rpc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

use crate::processor::v2::{
    common::UpdateEligibility,
    state::{
        helpers::{fetch_address_batches, fetch_address_zkp_batch_size},
        proof_worker::{spawn_proof_workers, ProofInput, ProofJob, ProofResult},
        tx_sender::TxSender,
    },
    BatchContext,
};

#[derive(Debug, Clone)]
pub struct AddressQueueWork {
    pub queue_size: u64,
}

#[derive(Debug, Clone)]
pub struct ProcessAddressQueueUpdate {
    pub work: AddressQueueWork,
}

struct WorkerPool {
    job_tx: async_channel::Sender<ProofJob>,
}

pub struct AddressSupervisor<R: Rpc> {
    context: BatchContext<R>,
    staging_tree: Option<AddressStagingTree>,
    current_root: [u8; 32],
    zkp_batch_size: u64,
    worker_pool: Option<WorkerPool>,
}

impl<R: Rpc> Actor for AddressSupervisor<R> {
    type Args = BatchContext<R>;
    type Error = anyhow::Error;

    async fn on_start(
        context: Self::Args,
        _actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        info!(
            "AddressSupervisor actor starting for tree {}",
            context.merkle_tree
        );

        let zkp_batch_size = fetch_address_zkp_batch_size(&context).await.map_err(|e| {
            error!(
                "Failed to fetch zkp_batch_size for tree {}: {}",
                context.merkle_tree, e
            );
            e
        })?;
        info!(
            "AddressSupervisor fetched zkp_batch_size={} for tree {}",
            zkp_batch_size, context.merkle_tree
        );

        Ok(Self {
            context,
            staging_tree: None,
            current_root: [0u8; 32],
            zkp_batch_size,
            worker_pool: None,
        })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        info!(
            "AddressSupervisor actor stopping for tree {}",
            self.context.merkle_tree
        );
        Ok(())
    }
}

impl<R: Rpc> Message<ProcessAddressQueueUpdate> for AddressSupervisor<R> {
    type Reply = crate::Result<usize>;

    async fn handle(
        &mut self,
        msg: ProcessAddressQueueUpdate,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.process_queue_update(msg.work).await
    }
}

impl<R: Rpc> Message<UpdateEligibility> for AddressSupervisor<R> {
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

impl<R: Rpc> AddressSupervisor<R> {
    fn zkp_batch_size(&self) -> u64 {
        self.zkp_batch_size
    }

    fn ensure_worker_pool(&mut self) {
        if self.worker_pool.is_none() {
            let num_workers = self.context.num_proof_workers.max(1);
            let job_tx = spawn_proof_workers(num_workers, self.context.prover_config.clone());
            self.worker_pool = Some(WorkerPool { job_tx });
        }
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
        subtrees: &[[u8; 32]],
        start_index: usize,
        initial_root: [u8; 32],
    ) -> crate::Result<()> {
        self.staging_tree = Some(AddressStagingTree::from_subtrees_vec(
            subtrees.to_vec(),
            start_index,
            initial_root,
        )?);
        self.current_root = initial_root;
        debug!("Built staging tree from indexer (root={:?}[..4])", &initial_root[..4]);
        Ok(())
    }

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

    fn batch_range(&self, total_len: usize, start: usize) -> std::ops::Range<usize> {
        let end = (start + self.zkp_batch_size as usize).min(total_len);
        start..end
    }

    fn create_job(
        seq: u64,
        inputs: ProofInput,
        result_tx: mpsc::Sender<ProofResult>,
    ) -> ProofJob {
        ProofJob {
            seq,
            inputs,
            result_tx,
        }
    }

    async fn process_queue_update(&mut self, work: AddressQueueWork) -> crate::Result<usize> {
        debug!(
            "AddressSupervisor processing queue update for tree {}",
            self.context.merkle_tree
        );

        let zkp_batch_size = self.zkp_batch_size();
        if work.queue_size < zkp_batch_size {
            trace!(
                "Queue size {} below zkp_batch_size {}, skipping",
                work.queue_size,
                zkp_batch_size
            );
            return Ok(0);
        }

        let max_batches = (work.queue_size / zkp_batch_size) as usize;
        if max_batches == 0 {
            return Ok(0);
        }

        self.ensure_worker_pool();

        let num_workers = self.context.num_proof_workers.max(1);
        let (proof_tx, proof_rx) = mpsc::channel(num_workers * 2);

        // Spawn tx sender with the current root
        let tx_sender_handle = TxSender::spawn(
            self.context.clone(),
            proof_rx,
            self.zkp_batch_size(),
            self.current_root,
        );

        let job_tx = self
            .worker_pool
            .as_ref()
            .expect("worker pool should be initialized")
            .job_tx
            .clone();

        // Build and send jobs one at a time
        let jobs_sent = self
            .enqueue_batches(max_batches, job_tx, proof_tx.clone())
            .await?;

        // Drop proof_tx to signal no more proofs are coming
        drop(proof_tx);

        // Wait for all transactions to complete
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

        if tx_processed < jobs_sent * self.zkp_batch_size as usize {
            debug!(
                "Processed {} items but expected {}, some proofs may have failed",
                tx_processed,
                jobs_sent * self.zkp_batch_size as usize
            );
        }

        Ok(tx_processed)
    }

    async fn enqueue_batches(
        &mut self,
        max_batches: usize,
        job_tx: async_channel::Sender<ProofJob>,
        result_tx: mpsc::Sender<ProofResult>,
    ) -> crate::Result<usize> {
        let zkp_batch_size = self.zkp_batch_size() as usize;
        let total_needed = max_batches.saturating_mul(zkp_batch_size);
        let fetch_len = total_needed as u64;

        debug!(
            "Fetching address batches: fetch_len={}, zkp_batch_size={}",
            fetch_len,
            self.zkp_batch_size()
        );
        let address_queue =
            fetch_address_batches(&self.context, None, fetch_len, self.zkp_batch_size()).await?;

        let Some(address_queue) = address_queue else {
            debug!("fetch_address_batches returned None, no address queue data available");
            return Ok(0);
        };

        if address_queue.addresses.is_empty() {
            debug!("Address queue is empty, returning");
            return Ok(0);
        }

        // Validate we have required data
        if address_queue.subtrees.is_empty() {
            return Err(anyhow!(
                "Address queue missing subtrees data (required for proof generation)"
            ));
        }

        // Calculate how many complete batches we can process
        let available = address_queue.addresses.len();
        let num_slices = (available / zkp_batch_size).min(max_batches);

        // If we can't form any complete batches, return early
        if num_slices == 0 {
            debug!(
                "Not enough addresses for a complete batch: have {}, need {}",
                available, zkp_batch_size
            );
            return Ok(0);
        }

        // Validate we have hash chains for the batches we'll process
        if address_queue.leaves_hash_chains.len() < num_slices {
            return Err(anyhow!(
                "Insufficient leaves_hash_chains: have {}, need {} for {} batches",
                address_queue.leaves_hash_chains.len(),
                num_slices,
                num_slices
            ));
        }

        // Build or update staging tree
        self.current_root = address_queue.initial_root;
        info!(
            "Synced from indexer: root {:?}[..4], start_index {}, {} subtrees",
            &self.current_root[..4],
            address_queue.start_index,
            address_queue.subtrees.len()
        );

        self.build_staging_tree(
            &address_queue.subtrees,
            address_queue.start_index as usize,
            address_queue.initial_root,
        )?;

        let mut jobs_sent = 0usize;

        // Generate circuit inputs and send jobs sequentially
        for batch_idx in 0..num_slices {
            let start = batch_idx * zkp_batch_size;
            if let Some(job) = self
                .build_append_job(batch_idx, &address_queue, start, result_tx.clone())
                .await?
            {
                job_tx.send(job).await?;
                jobs_sent += 1;
            } else {
                break;
            }
        }

        info!("Enqueued {} jobs for proof generation", jobs_sent);
        Ok(jobs_sent)
    }

    async fn build_append_job(
        &mut self,
        batch_idx: usize,
        address_queue: &light_client::indexer::AddressQueueDataV2,
        start: usize,
        result_tx: mpsc::Sender<ProofResult>,
    ) -> crate::Result<Option<ProofJob>> {
        let range = self.batch_range(address_queue.addresses.len(), start);
        let addresses = address_queue.addresses[range.clone()].to_vec();
        let zkp_batch_size = addresses.len();

        // Get data for this batch
        let low_element_values = address_queue.low_element_values[range.clone()].to_vec();
        let low_element_next_values = address_queue.low_element_next_values[range.clone()].to_vec();
        let low_element_indices: Vec<usize> = address_queue.low_element_indices[range.clone()]
            .iter()
            .map(|&i| i as usize)
            .collect();
        let low_element_next_indices: Vec<usize> = address_queue.low_element_next_indices
            [range.clone()]
        .iter()
        .map(|&i| i as usize)
        .collect();
        let low_element_proofs = address_queue.low_element_proofs[range].to_vec();

        // Get pre-computed hash chain for this batch
        let leaves_hashchain =
            Self::get_leaves_hashchain(&address_queue.leaves_hash_chains, batch_idx)?;

        // Get mutable reference to staging tree
        let staging_tree = self.staging_tree.as_mut().ok_or_else(|| {
            anyhow!(
                "Staging tree not initialized for append job (batch_idx={})",
                batch_idx
            )
        })?;

        // Process batch using AddressStagingTree which internally uses
        // get_batch_address_append_circuit_inputs with proper changelog management
        let result = staging_tree
            .process_batch(
                addresses,
                low_element_values,
                low_element_next_values,
                low_element_indices,
                low_element_next_indices,
                low_element_proofs,
                leaves_hashchain,
                zkp_batch_size,
            )
            .map_err(|e| anyhow!("Failed to process address batch: {}", e))?;

        let new_root = result.new_root;
        debug!(
            "Address batch {} root transition: {:?}[..4] -> {:?}[..4]",
            batch_idx,
            &result.old_root[..4],
            &new_root[..4]
        );

        // Update current root
        self.current_root = new_root;

        Ok(Some(Self::create_job(
            batch_idx as u64,
            ProofInput::AddressAppend(result.circuit_inputs),
            result_tx,
        )))
    }
}
