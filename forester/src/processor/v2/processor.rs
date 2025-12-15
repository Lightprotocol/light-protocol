use std::{
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};

use anyhow::anyhow;
use forester_utils::{forester_epoch::EpochPhases, utils::wait_for_indexer};
use light_client::rpc::Rpc;
use light_compressed_account::QueueType;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::{
    epoch_manager::{CircuitMetrics, ProcessingMetrics},
    processor::v2::{
        common::WorkerPool,
        proof_cache::SharedProofCache,
        proof_worker::{spawn_proof_workers, ProofJob, ProofJobResult},
        strategy::{CircuitType, QueueData, TreeStrategy},
        tx_sender::TxSender,
        BatchContext, ProcessingResult, QueueWork,
    },
};

const MAX_BATCHES_PER_TREE: usize = 20;

/// Tracks timing and counts per circuit type for accurate metrics
#[derive(Debug, Default, Clone)]
struct BatchTimings {
    /// Circuit input generation time per circuit type
    append_circuit_inputs: Duration,
    nullify_circuit_inputs: Duration,
    address_append_circuit_inputs: Duration,
    /// Batch counts per circuit type
    append_count: usize,
    nullify_count: usize,
    address_append_count: usize,
}

impl BatchTimings {
    fn add_timing(&mut self, circuit_type: CircuitType, duration: Duration) {
        match circuit_type {
            CircuitType::Append => {
                self.append_circuit_inputs += duration;
                self.append_count += 1;
            }
            CircuitType::Nullify => {
                self.nullify_circuit_inputs += duration;
                self.nullify_count += 1;
            }
            CircuitType::AddressAppend => {
                self.address_append_circuit_inputs += duration;
                self.address_append_count += 1;
            }
        }
    }
}

/// Cached queue state for optimistic processing - allows continuing without re-fetching from indexer
struct CachedQueueState<T> {
    /// The staging tree with state after last processed batch
    staging_tree: T,
    /// Number of batches already processed from this data
    batches_processed: usize,
    /// Total batches available in this queue data
    total_batches: usize,
}

pub struct QueueProcessor<R: Rpc, S: TreeStrategy<R>> {
    context: BatchContext<R>,
    strategy: S,
    current_root: [u8; 32],
    zkp_batch_size: u64,
    seq: u64,
    worker_pool: Option<WorkerPool>,
    /// Cached queue state for optimistic processing - continue without waiting for indexer
    cached_state: Option<CachedQueueState<S::StagingTree>>,
    /// Optional proof cache for saving unused proofs when epoch ends
    proof_cache: Option<Arc<SharedProofCache>>,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: Rpc, S: TreeStrategy<R>> std::fmt::Debug for QueueProcessor<R, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueProcessor")
            .field("merkle_tree", &self.context.merkle_tree)
            .field("epoch", &self.context.epoch)
            .field("zkp_batch_size", &self.zkp_batch_size)
            .finish()
    }
}

impl<R: Rpc, S: TreeStrategy<R> + 'static> QueueProcessor<R, S> {
    pub async fn new(context: BatchContext<R>, strategy: S) -> crate::Result<Self> {
        let zkp_batch_size = strategy.fetch_zkp_batch_size(&context).await?;
        // Initialize with on-chain root to ensure we don't accept stale indexer data
        let current_root = strategy.fetch_onchain_root(&context).await?;
        info!(
            "Initializing {} processor for tree {} with on-chain root {:?}[..4]",
            strategy.name(),
            context.merkle_tree,
            &current_root[..4]
        );
        Ok(Self {
            context,
            strategy,
            current_root,
            zkp_batch_size,
            seq: 0,
            worker_pool: None,
            cached_state: None,
            proof_cache: None,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Set the proof cache for saving unused proofs when epoch ends
    pub fn set_proof_cache(&mut self, cache: Arc<SharedProofCache>) {
        self.proof_cache = Some(cache);
    }

    pub async fn process_queue_update(
        &mut self,
        queue_work: QueueWork,
    ) -> crate::Result<ProcessingResult> {
        if queue_work.queue_size < self.zkp_batch_size {
            return Ok(ProcessingResult::default());
        }

        if self.worker_pool.is_none() {
            let job_tx = spawn_proof_workers(&self.context.prover_config);
            self.worker_pool = Some(WorkerPool { job_tx });
        }

        // OPTIMISTIC PATH: Check if we have cached state from previous iteration
        // This allows us to continue processing without waiting for indexer to sync
        if let Some(cached) = self.cached_state.take() {
            let remaining = cached
                .total_batches
                .saturating_sub(cached.batches_processed);
            if remaining > 0 {
                info!(
                    "Using cached state: {} remaining batches (processed {}/{})",
                    remaining, cached.batches_processed, cached.total_batches
                );

                // Determine how many batches to process this iteration
                let batches_to_process = remaining.min(MAX_BATCHES_PER_TREE);

                // Build queue_data from cached state
                let queue_data = QueueData {
                    staging_tree: cached.staging_tree,
                    initial_root: self.current_root,
                    num_batches: cached.total_batches,
                };

                // batch_offset is where we left off
                return self
                    .process_batches(
                        queue_data,
                        cached.batches_processed,
                        batches_to_process,
                        cached.total_batches,
                    )
                    .await;
            }
        }

        // FETCH PATH: No cached state, need to fetch from indexer
        // Fetch multiple iterations worth of batches - paginated fetch handles
        // timeout by making multiple small requests internally
        let available_batches = (queue_work.queue_size / self.zkp_batch_size) as usize;
        let fetch_batches = available_batches.min(MAX_BATCHES_PER_TREE);

        if available_batches > MAX_BATCHES_PER_TREE {
            debug!(
                "Queue has {} batches available, fetching {} for {} iterations",
                available_batches,
                fetch_batches,
                fetch_batches.div_ceil(MAX_BATCHES_PER_TREE)
            );
        }

        // Wait for indexer to catch up with RPC before fetching
        {
            let rpc = self.context.rpc_pool.get_connection().await?;
            if let Err(e) = wait_for_indexer(&*rpc).await {
                warn!("wait_for_indexer error (proceeding anyway): {}", e);
            }
        }

        // Fetch queue data - paginated internally to avoid timeout
        let queue_data = match self
            .strategy
            .fetch_queue_data(
                &self.context,
                &queue_work,
                fetch_batches,
                self.zkp_batch_size,
            )
            .await?
        {
            Some(data) => data,
            None => return Ok(ProcessingResult::default()),
        };

        // Check if indexer root matches our expected root
        if self.current_root == [0u8; 32] || queue_data.initial_root == self.current_root {
            // Roots match - process batches, caching remaining for next iteration
            let total_batches = queue_data.num_batches;
            let process_now = total_batches.min(MAX_BATCHES_PER_TREE);
            return self
                .process_batches(queue_data, 0, process_now, total_batches)
                .await;
        }

        // Root mismatch - check on-chain to decide what to do
        let onchain_root = self.strategy.fetch_onchain_root(&self.context).await?;
        if onchain_root == self.current_root {
            // On-chain matches our expected root - indexer is behind
            debug!(
                "Indexer root {:?}[..4] doesn't match expected {:?}[..4], on-chain confirms we're ahead. Waiting for next slot.",
                &queue_data.initial_root[..4],
                &self.current_root[..4]
            );
            return Ok(ProcessingResult::default());
        }

        // On-chain root differs - accept indexer data with on-chain root
        if queue_data.initial_root == onchain_root {
            debug!(
                "Resetting to on-chain root {:?}[..4] (was expecting {:?}[..4])",
                &onchain_root[..4],
                &self.current_root[..4]
            );
            self.current_root = onchain_root;
            self.cached_state = None;
            let total_batches = queue_data.num_batches;
            let process_now = total_batches.min(MAX_BATCHES_PER_TREE);
            return self
                .process_batches(queue_data, 0, process_now, total_batches)
                .await;
        }

        // Neither matches - something is very wrong, reset and wait
        warn!(
            "Root divergence: expected {:?}[..4], indexer {:?}[..4], on-chain {:?}[..4]. Resetting.",
            &self.current_root[..4],
            &queue_data.initial_root[..4],
            &onchain_root[..4]
        );
        self.current_root = onchain_root;
        self.cached_state = None;
        Ok(ProcessingResult::default())
    }

    /// Clear cached state - call this when root divergence is detected or on error
    pub fn clear_cache(&mut self) {
        self.cached_state = None;
    }

    pub fn update_eligibility(&mut self, end_slot: u64) {
        self.context
            .forester_eligibility_end_slot
            .store(end_slot, Ordering::Relaxed);
    }

    /// Update the epoch for this processor (called when reusing across epoch transitions)
    pub fn update_epoch(&mut self, new_epoch: u64, new_phases: EpochPhases) {
        self.context.epoch = new_epoch;
        self.context.epoch_phases = new_phases;
    }

    pub fn merkle_tree(&self) -> &Pubkey {
        &self.context.merkle_tree
    }

    pub fn epoch(&self) -> u64 {
        self.context.epoch
    }

    pub fn zkp_batch_size(&self) -> u64 {
        self.zkp_batch_size
    }

    /// Process batches from queue data.
    /// `batch_offset`: starting batch index (for continuation from cached state)
    /// `batches_to_process`: how many batches to process in this iteration
    /// `total_batches`: total batches available in queue_data (for caching remaining)
    async fn process_batches(
        &mut self,
        queue_data: QueueData<S::StagingTree>,
        batch_offset: usize,
        batches_to_process: usize,
        total_batches: usize,
    ) -> crate::Result<ProcessingResult> {
        self.current_root = queue_data.initial_root;
        let num_workers = self.context.num_proof_workers.max(1);
        let (proof_tx, proof_rx) = mpsc::channel(num_workers * 2);

        self.seq = 0;
        let proof_gen_start = Instant::now();
        let tx_sender_handle = TxSender::spawn(
            self.context.clone(),
            proof_rx,
            self.zkp_batch_size,
            self.current_root,
            self.proof_cache.clone(),
        );
        let job_tx = self
            .worker_pool
            .as_ref()
            .ok_or_else(|| anyhow!("Worker pool not initialized"))?
            .job_tx
            .clone();

        let (jobs_sent, timings, staging_tree) = self
            .enqueue_jobs(
                queue_data,
                batch_offset,
                batches_to_process,
                job_tx,
                proof_tx.clone(),
            )
            .await?;

        // Cache remaining batches for optimistic continuation
        let total_processed = batch_offset + batches_to_process;
        let remaining_batches = total_batches.saturating_sub(total_processed);
        if remaining_batches > 0 {
            debug!(
                "Caching {} remaining batches for optimistic continuation (processed {}/{})",
                remaining_batches, total_processed, total_batches
            );
            self.cached_state = Some(CachedQueueState {
                staging_tree,
                batches_processed: total_processed,
                total_batches,
            });
        } else {
            // No remaining batches, clear cache
            self.cached_state = None;
        }

        drop(proof_tx);

        let tx_result = tx_sender_handle
            .await
            .map_err(|e| anyhow!("Tx sender join error: {}", e))
            .and_then(|res| res);

        let (tx_processed, proof_timings) = match &tx_result {
            Ok(result) => (result.items_processed, result.proof_timings.clone()),
            Err(e) => {
                let err_str = e.to_string();
                warn!(
                    "Tx sender error for tree {}: {}",
                    self.context.merkle_tree, err_str
                );

                if is_constraint_error(&err_str) {
                    // Surface constraint errors so the caller can reset/flush state.
                    return Err(anyhow!("Constraint error during tx send: {}", err_str));
                }

                (0, Default::default())
            }
        };

        let _total_time = proof_gen_start.elapsed();

        if tx_processed < jobs_sent * self.zkp_batch_size as usize {
            debug!(
                "Processed {} items but expected {}, some proofs may have failed",
                tx_processed,
                jobs_sent * self.zkp_batch_size as usize
            );
        }

        // Build metrics using actual measured timings:
        // - Circuit inputs: from BatchTimings (measured per-batch during enqueue_jobs)
        // - Proof generation: from ProofTimings (actual server times from prover)
        let mut metrics = ProcessingMetrics::default();

        if timings.append_count > 0 {
            metrics.append = CircuitMetrics {
                circuit_inputs_duration: timings.append_circuit_inputs,
                proof_generation_duration: proof_timings.append_proof_duration(),
                round_trip_duration: proof_timings.append_round_trip_duration(),
            };
        }
        if timings.nullify_count > 0 {
            metrics.nullify = CircuitMetrics {
                circuit_inputs_duration: timings.nullify_circuit_inputs,
                proof_generation_duration: proof_timings.nullify_proof_duration(),
                round_trip_duration: proof_timings.nullify_round_trip_duration(),
            };
        }
        if timings.address_append_count > 0 {
            metrics.address_append = CircuitMetrics {
                circuit_inputs_duration: timings.address_append_circuit_inputs,
                proof_generation_duration: proof_timings.address_append_proof_duration(),
                round_trip_duration: proof_timings.address_append_round_trip_duration(),
            };
        }

        // Always return metrics so timing data is captured even on error
        // Log the error but don't propagate it - partial progress is still progress
        if let Err(e) = tx_result {
            warn!(
                "Returning partial metrics despite error for tree {}: {}",
                self.context.merkle_tree, e
            );
        }

        Ok(ProcessingResult {
            items_processed: tx_processed,
            metrics,
        })
    }

    /// Enqueue proof generation jobs on a blocking thread pool.
    /// Takes ownership of queue_data and returns the staging tree along with jobs_sent count.
    /// This allows the proof worker to process jobs concurrently while inputs are generated.
    /// The staging tree is returned so it can be cached for optimistic continuation.
    ///
    /// `batch_offset`: starting batch index (for continuation from cached state)
    /// `num_batches`: how many batches to process starting from batch_offset
    async fn enqueue_jobs(
        &mut self,
        queue_data: QueueData<S::StagingTree>,
        batch_offset: usize,
        num_batches: usize,
        job_tx: async_channel::Sender<ProofJob>,
        result_tx: mpsc::Sender<ProofJobResult>,
    ) -> crate::Result<(usize, BatchTimings, S::StagingTree)>
    where
        S::StagingTree: 'static,
    {
        let zkp_batch_size = self.zkp_batch_size;
        let zkp_batch_size_usize = zkp_batch_size as usize;
        let strategy = self.strategy.clone();
        let initial_seq = self.seq;
        let epoch = self.context.epoch;
        let tree = self.context.merkle_tree.to_string();

        // Run CPU-bound proof input generation on blocking thread pool
        // This allows the proof worker to process jobs concurrently
        let result = tokio::task::spawn_blocking(move || {
            let mut staging_tree = queue_data.staging_tree;
            let mut jobs_sent = 0;
            let mut final_root = queue_data.initial_root;
            let mut current_seq = initial_seq;
            let mut timings = BatchTimings::default();

            // Process batches starting from batch_offset
            for i in 0..num_batches {
                let batch_idx = batch_offset + i;
                let start = batch_idx * zkp_batch_size_usize;

                // Track circuit type for this specific batch (accounts for combined APPEND+NULLIFY)
                let circuit_type = strategy.circuit_type_for_batch(&staging_tree, batch_idx);

                // Measure circuit input generation time per-batch
                let circuit_start = Instant::now();
                let (inputs, new_root) = strategy.build_proof_job(
                    &mut staging_tree,
                    batch_idx,
                    start,
                    zkp_batch_size,
                    epoch,
                    &tree,
                )?;
                let circuit_duration = circuit_start.elapsed();

                // Track timing and count by circuit type
                timings.add_timing(circuit_type, circuit_duration);

                final_root = new_root;
                let job = ProofJob {
                    seq: current_seq,
                    inputs,
                    result_tx: result_tx.clone(),
                    tree_id: tree.clone(),
                };
                current_seq += 1;

                // Use blocking send since we're in a sync context
                job_tx
                    .send_blocking(job)
                    .map_err(|e| anyhow::anyhow!("Failed to send job: {}", e))?;
                jobs_sent += 1;
            }

            // Return staging_tree for optimistic caching
            Ok::<_, anyhow::Error>((jobs_sent, final_root, current_seq, timings, staging_tree))
        })
        .await
        .map_err(|e| anyhow::anyhow!("Blocking task panicked: {}", e))??;

        let (jobs_sent, final_root, final_seq, timings, staging_tree) = result;

        self.current_root = final_root;
        self.seq = final_seq;

        Ok((jobs_sent, timings, staging_tree))
    }

    /// Pre-warm the proof cache by generating proofs without sending transactions.
    /// This should be called during idle time (when forester is not eligible).
    /// Returns the number of proofs cached.
    pub async fn prewarm_proofs(
        &mut self,
        cache: Arc<SharedProofCache>,
        queue_work: QueueWork,
    ) -> crate::Result<usize> {
        if queue_work.queue_size < self.zkp_batch_size {
            return Ok(0);
        }

        let max_batches =
            ((queue_work.queue_size / self.zkp_batch_size) as usize).min(MAX_BATCHES_PER_TREE);

        if self.worker_pool.is_none() {
            let job_tx = spawn_proof_workers(&self.context.prover_config);
            self.worker_pool = Some(WorkerPool { job_tx });
        }

        let queue_data = match self
            .strategy
            .fetch_queue_data(&self.context, &queue_work, max_batches, self.zkp_batch_size)
            .await?
        {
            Some(data) => data,
            None => return Ok(0),
        };

        self.prewarm_batches(cache, queue_data).await
    }

    /// Pre-warm proofs by fetching queue data directly from the indexer without
    /// consuming queue update messages. Useful when we don't want to drain
    /// queue_update channels (e.g., address queues).
    pub async fn prewarm_from_indexer(
        &mut self,
        cache: Arc<SharedProofCache>,
        queue_type: QueueType,
        max_batches: usize,
    ) -> crate::Result<usize> {
        if max_batches == 0 {
            return Ok(0);
        }

        let max_batches = max_batches.min(MAX_BATCHES_PER_TREE);
        let queue_size_hint = (self.zkp_batch_size as usize * max_batches) as u64;

        if self.worker_pool.is_none() {
            let job_tx = spawn_proof_workers(&self.context.prover_config);
            self.worker_pool = Some(WorkerPool { job_tx });
        }

        let queue_work = QueueWork {
            queue_type,
            queue_size: queue_size_hint,
        };

        let queue_data = match self
            .strategy
            .fetch_queue_data(&self.context, &queue_work, max_batches, self.zkp_batch_size)
            .await?
        {
            Some(data) => data,
            None => return Ok(0),
        };

        self.prewarm_batches(cache, queue_data).await
    }

    /// Generate proofs and cache them instead of sending transactions
    async fn prewarm_batches(
        &mut self,
        cache: Arc<SharedProofCache>,
        queue_data: QueueData<S::StagingTree>,
    ) -> crate::Result<usize> {
        let initial_root = queue_data.initial_root;
        self.current_root = initial_root;
        let num_batches = queue_data.num_batches;
        let num_workers = self.context.num_proof_workers.max(1);

        // Start cache warming with the current root
        cache.start_warming(initial_root).await;

        let (proof_tx, mut proof_rx) = mpsc::channel(num_workers * 2);

        self.seq = 0;
        let job_tx = self
            .worker_pool
            .as_ref()
            .ok_or_else(|| anyhow!("Worker pool not initialized"))?
            .job_tx
            .clone();

        info!(
            "Pre-warming {} proofs for tree {} with root {:?}",
            num_batches,
            self.context.merkle_tree,
            &initial_root[..4]
        );

        // For prewarming, we always start from batch 0 and don't cache the result
        let (jobs_sent, _, _staging_tree) = self
            .enqueue_jobs(queue_data, 0, num_batches, job_tx, proof_tx.clone())
            .await?;

        drop(proof_tx);

        // Collect proofs into cache instead of sending transactions
        let mut proofs_cached = 0;
        while let Some(result) = proof_rx.recv().await {
            match result.result {
                Ok(instruction) => {
                    cache
                        .add_proof(result.seq, result.old_root, result.new_root, instruction)
                        .await;
                    proofs_cached += 1;
                }
                Err(e) => {
                    warn!(
                        "Proof generation failed during pre-warm for seq={}: {}",
                        result.seq, e
                    );
                    // Continue collecting other proofs
                }
            }
        }

        cache.finish_warming().await;

        if proofs_cached < jobs_sent {
            warn!(
                "Pre-warmed {} proofs but expected {} for tree {}",
                proofs_cached, jobs_sent, self.context.merkle_tree
            );
        } else {
            info!(
                "Pre-warmed {} proofs for tree {} (zkp_batch_size={}, items={})",
                proofs_cached,
                self.context.merkle_tree,
                self.zkp_batch_size,
                proofs_cached * self.zkp_batch_size as usize
            );
        }

        Ok(proofs_cached)
    }

    /// Get the current root that proofs are being generated against
    pub fn current_root(&self) -> &[u8; 32] {
        &self.current_root
    }
}

pub fn is_constraint_error(msg: &str) -> bool {
    let lower = msg.to_ascii_lowercase();
    lower.contains("constraint")
        || lower.contains("unsatisfied")
        // ProofVerificationFailed (error code 13006 / 0x32ce) - staging tree is out of sync
        || lower.contains("0x32ce")
        || lower.contains("13006")
}

/// Check if an error is a hashchain mismatch - indicates stale cached data
pub fn is_hashchain_mismatch(msg: &str) -> bool {
    msg.contains("HASHCHAIN MISMATCH")
}
