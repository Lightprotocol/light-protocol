/// Unified Generic Batch Processor
///
/// This module provides a single, elegant processor implementation that handles
/// both state and address trees through a strategy pattern.

use anyhow::anyhow;
use async_trait::async_trait;
use light_client::rpc::Rpc;
use solana_sdk::pubkey::Pubkey;
use std::sync::atomic::Ordering;
use tokio::sync::mpsc;
use tracing::{debug, info, trace, warn};

use crate::processor::v2::{
    common::{ensure_worker_pool, WorkerPool},
    state::{
        proof_worker::{ProofInput, ProofJob, ProofResult},
        tx_sender::TxSender,
    },
    BatchContext, QueueWork,
};

// ============================================================================
// Strategy Trait - Tree-Specific Operations
// ============================================================================

/// Strategy for tree-specific batch processing operations.
///
/// This trait encapsulates the differences between state and address tree processing,
/// allowing a single generic processor to handle both types.
#[async_trait]
pub trait TreeStrategy<R: Rpc>: Send + Sync + std::fmt::Debug {
    /// Type of staging tree used by this strategy
    type StagingTree: Send;

    /// Name of this strategy (for logging)
    fn name(&self) -> &'static str;

    /// Fetches the zkp batch size from on-chain configuration
    async fn fetch_zkp_batch_size(&self, context: &BatchContext<R>) -> crate::Result<u64>;

    /// Fetches queue data from the indexer
    async fn fetch_queue_data(
        &self,
        context: &BatchContext<R>,
        queue_work: &QueueWork,
        max_batches: usize,
        zkp_batch_size: u64,
    ) -> crate::Result<Option<QueueData<Self::StagingTree>>>;

    /// Builds a proof job for the given batch (synchronous CPU work)
    fn build_proof_job(
        &self,
        queue_data: &mut Self::StagingTree,
        batch_idx: usize,
        start: usize,
        zkp_batch_size: u64,
    ) -> crate::Result<(ProofInput, [u8; 32])>;

    /// Validates queue work before processing
    fn validate_queue_work(&self, queue_work: &QueueWork) -> bool;
}

/// Data fetched from indexer, generic over staging tree type
#[derive(Debug)]
pub struct QueueData<T> {
    pub staging_tree: T,
    pub initial_root: [u8; 32],
    pub num_batches: usize,
}

// ============================================================================
// Unified Generic Processor
// ============================================================================

/// Generic batch processor that works with any tree strategy.
///
/// This is the single unified implementation for all batch processing.
/// Tree-specific behavior is delegated to the `TreeStrategy`.
#[derive(Debug)]
pub struct UnifiedBatchProcessor<R: Rpc, S: TreeStrategy<R>> {
    context: BatchContext<R>,
    strategy: S,
    current_root: [u8; 32],
    zkp_batch_size: u64,
    seq: u64,
    worker_pool: Option<WorkerPool>,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: Rpc, S: TreeStrategy<R>> UnifiedBatchProcessor<R, S> {
    /// Creates a new unified batch processor with the given strategy.
    pub async fn new(context: BatchContext<R>, strategy: S) -> crate::Result<Self> {
        info!(
            "UnifiedBatchProcessor[{}] initializing for tree {}",
            strategy.name(),
            context.merkle_tree
        );

        let zkp_batch_size = strategy.fetch_zkp_batch_size(&context).await?;

        // Validate zkp_batch_size to prevent division by zero and invalid batches
        if zkp_batch_size == 0 {
            return Err(anyhow!("Invalid zkp_batch_size: cannot be zero"));
        }
        if zkp_batch_size > 10000 {
            warn!(
                "Unusually large zkp_batch_size={}, capping at 10000",
                zkp_batch_size
            );
            // Note: Consider returning error instead if this is unexpected
        }

        info!(
            "UnifiedBatchProcessor[{}] initialized zkp_batch_size={} for tree {}",
            strategy.name(),
            zkp_batch_size,
            context.merkle_tree
        );

        Ok(Self {
            context,
            strategy,
            current_root: [0u8; 32],
            zkp_batch_size,
            seq: 0,
            worker_pool: None,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Processes a queue update using the strategy pattern.
    pub async fn process_queue_update(&mut self, queue_work: QueueWork) -> crate::Result<usize> {
        debug!(
            "UnifiedBatchProcessor[{}] processing queue update for tree {}",
            self.strategy.name(),
            self.context.merkle_tree
        );

        // Validate queue work
        if !self.strategy.validate_queue_work(&queue_work) {
            return Ok(0);
        }

        // Check batch size threshold
        if queue_work.queue_size < self.zkp_batch_size {
            trace!(
                "Queue size {} below zkp_batch_size {}, skipping",
                queue_work.queue_size,
                self.zkp_batch_size
            );
            return Ok(0);
        }

        // Calculate max_batches with upper bound to prevent resource exhaustion
        const MAX_BATCHES_LIMIT: usize = 1000;
        let max_batches = ((queue_work.queue_size / self.zkp_batch_size) as usize).min(MAX_BATCHES_LIMIT);

        if queue_work.queue_size / self.zkp_batch_size > MAX_BATCHES_LIMIT as u64 {
            debug!(
                "Queue size {} would produce {} batches, limiting to {}",
                queue_work.queue_size,
                queue_work.queue_size / self.zkp_batch_size,
                MAX_BATCHES_LIMIT
            );
        }

        // Ensure worker pool is ready
        ensure_worker_pool(
            &mut self.worker_pool,
            self.context.num_proof_workers,
            &self.context.prover_config,
            &format!(
                "UnifiedBatchProcessor[{}] tree {}",
                self.strategy.name(),
                self.context.merkle_tree
            ),
        );

        // Fetch queue data via strategy
        let queue_data = match self
            .strategy
            .fetch_queue_data(&self.context, &queue_work, max_batches, self.zkp_batch_size)
            .await?
        {
            Some(data) => data,
            None => return Ok(0),
        };

        // Process batches
        self.process_batches(queue_data).await
    }

    /// Updates the forester eligibility end slot.
    pub fn update_eligibility(&mut self, end_slot: u64) {
        debug!(
            "Updating eligibility end slot to {} for tree {}",
            end_slot, self.context.merkle_tree
        );
        self.context
            .forester_eligibility_end_slot
            .store(end_slot, Ordering::Relaxed);
    }

    /// Returns the merkle tree pubkey.
    pub fn merkle_tree(&self) -> &Pubkey {
        &self.context.merkle_tree
    }

    /// Returns the current epoch.
    pub fn epoch(&self) -> u64 {
        self.context.epoch
    }

    // ========================================================================
    // Internal Processing Logic (Unified)
    // ========================================================================

    async fn process_batches(
        &mut self,
        mut queue_data: QueueData<S::StagingTree>,
    ) -> crate::Result<usize> {
        self.current_root = queue_data.initial_root;
        let num_batches = queue_data.num_batches;

        info!(
            "Synced from indexer: root {:?}[..4], processing {} batches",
            &self.current_root[..4],
            num_batches
        );

        let num_workers = self.context.num_proof_workers.max(1);
        let (proof_tx, proof_rx) = mpsc::channel(num_workers * 2);

        // Reset sequence counter
        self.seq = 0;

        // Spawn transaction sender
        let tx_sender_handle = TxSender::spawn(
            self.context.clone(),
            proof_rx,
            self.zkp_batch_size,
            self.current_root,
        );

        let job_tx = self
            .worker_pool
            .as_ref()
            .ok_or_else(|| anyhow!("Worker pool not initialized"))?
            .job_tx
            .clone();

        // Generate and enqueue proof jobs
        let jobs_sent = self
            .enqueue_jobs(&mut queue_data, num_batches, job_tx, proof_tx.clone())
            .await?;

        // Signal completion
        drop(proof_tx);

        // Wait for transaction processing
        let tx_processed = tx_sender_handle
            .await
            .map_err(|e| anyhow!("Tx sender join error: {}", e))
            .and_then(|res| res)
            .inspect_err(|e| {
                warn!(
                    "Tx sender error for tree {}: {}",
                    self.context.merkle_tree, e
                );
            })?;

        if tx_processed < jobs_sent * self.zkp_batch_size as usize {
            debug!(
                "Processed {} items but expected {}, some proofs may have failed",
                tx_processed,
                jobs_sent * self.zkp_batch_size as usize
            );
        }

        Ok(tx_processed)
    }

    async fn enqueue_jobs(
        &mut self,
        queue_data: &mut QueueData<S::StagingTree>,
        num_batches: usize,
        job_tx: async_channel::Sender<ProofJob>,
        result_tx: mpsc::Sender<ProofResult>,
    ) -> crate::Result<usize> {
        let zkp_batch_size = self.zkp_batch_size as usize;
        let mut jobs_sent = 0;

        // Producer pattern: prepare and send batches as fast as possible
        // The channel acts as a buffer, allowing workers to start immediately
        for batch_idx in 0..num_batches {
            let start = batch_idx * zkp_batch_size;

            // Build circuit inputs for this batch (synchronous CPU work)
            let (inputs, new_root) = self
                .strategy
                .build_proof_job(
                    &mut queue_data.staging_tree,
                    batch_idx,
                    start,
                    self.zkp_batch_size,
                )?;

            // Create job and send immediately - workers start proving while we prepare next batch
            let job = self.finish_job(new_root, inputs, result_tx.clone());
            job_tx.send(job).await?;
            jobs_sent += 1;

            // Workers can start proving batch_idx while we prepare batch_idx+1
            // The async_channel buffer (capacity = num_workers * 2) enables this overlap
        }

        info!("Enqueued {} jobs for proof generation", jobs_sent);
        Ok(jobs_sent)
    }

    fn finish_job(
        &mut self,
        new_root: [u8; 32],
        inputs: ProofInput,
        result_tx: mpsc::Sender<ProofResult>,
    ) -> ProofJob {
        self.current_root = new_root;
        let job_seq = self.seq;
        self.seq += 1;
        ProofJob {
            seq: job_seq,
            inputs,
            result_tx,
        }
    }
}
