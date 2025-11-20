use light_prover_client::proof_types::{
    batch_append::BatchAppendInputsJson, batch_update::update_inputs_string,
};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use forester_utils::{forester_epoch::EpochPhases, ParsedMerkleTreeData, ParsedQueueData};
use light_batched_merkle_tree::{
    merkle_tree::{
        BatchedMerkleTreeAccount, InstructionDataBatchAppendInputs,
        InstructionDataBatchNullifyInputs,
    },
    queue::BatchedQueueAccount,
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::QueueType;
use light_prover_client::proof_client::ProofClient;
use once_cell::sync::Lazy;
use solana_sdk::{account::Account, pubkey::Pubkey};
use tokio::sync::Mutex as TokioMutex;
use tracing::{debug, error, info, trace, warn};

use super::{
    batch_preparation, batch_submission,
    batch_utils::{self, validate_root, MAX_COORDINATOR_RETRIES},
    error::CoordinatorError,
    proof_generation::ProofConfig,
    proof_utils,
    shared_state::{get_or_create_shared_state, ProcessedBatchId, SharedState},
    telemetry::IterationTelemetry,
    types::{AppendQueueData, BatchType, NullifyQueueData, PreparationState, PreparedBatch},
};
use crate::{grpc::QueueUpdateMessage, metrics, processor::v2::common::BatchContext};

/// Channel buffer size for proof generation pipeline.
/// Balances memory usage vs throughput for concurrent batch processing.
const PROOF_CHANNEL_BUFFER_SIZE: usize = 50;

/// Maximum number of batches that can be submitted in a single transaction.
/// Constrained by Solana transaction size limits.
const MAX_BATCHES_PER_TRANSACTION: usize = 4;

type PersistentTreeStatesCache = Arc<TokioMutex<HashMap<Pubkey, SharedState>>>;
type StagingTreeCache =
    Arc<TokioMutex<HashMap<Pubkey, (super::tree_state::StagingTree, [u8; 32])>>>;

static PERSISTENT_TREE_STATES: Lazy<PersistentTreeStatesCache> =
    Lazy::new(|| Arc::new(TokioMutex::new(HashMap::new())));

/// Global cache for staging trees, keyed by merkle tree pubkey only (survives across epochs).
/// Stores (staging_tree, last_known_root) for each tree.
static PERSISTENT_STAGING_TREES: Lazy<StagingTreeCache> =
    Lazy::new(|| Arc::new(TokioMutex::new(HashMap::new())));

struct QueueFetchResult {
    staging: super::tree_state::StagingTree,
    output_queue_v2: Option<light_client::indexer::OutputQueueDataV2>,
    input_queue_v2: Option<light_client::indexer::InputQueueDataV2>,
    on_chain_root: [u8; 32],
    append_data: Option<AppendQueueData>,
    nullify_data: Option<NullifyQueueData>,
    append_batch_ids: Vec<ProcessedBatchId>,
    nullify_batch_ids: Vec<ProcessedBatchId>,
}

struct OnchainState {
    current_onchain_root: [u8; 32],
    append_metadata: Option<(ParsedMerkleTreeData, ParsedQueueData)>,
    nullify_metadata: Option<ParsedMerkleTreeData>,
    append_batch_ids: Vec<ProcessedBatchId>,
    nullify_batch_ids: Vec<ProcessedBatchId>,
}

enum ProofResult {
    Append(light_batched_merkle_tree::merkle_tree::InstructionDataBatchAppendInputs),
    Nullify(light_batched_merkle_tree::merkle_tree::InstructionDataBatchNullifyInputs),
}

pub struct StateTreeCoordinator<R: Rpc> {
    shared_state: SharedState,
    pub context: BatchContext<R>,
    current_light_slot: Option<u64>,
    /// Cached staging tree to preserve across iteration.
    /// Stores (staging_tree, root).
    cached_staging: Option<(super::tree_state::StagingTree, [u8; 32])>,
    pending_queue_items: usize,
    pending_append_items: usize,
    pending_nullify_items: usize,
    speculative: SpeculativeEngine,
}

impl<R: Rpc> fmt::Debug for StateTreeCoordinator<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateTreeCoordinator")
            .field("tree", &self.context.merkle_tree)
            .finish()
    }
}

impl<R: Rpc> StateTreeCoordinator<R> {
    fn record_cache_event(&self, event: &'static str, reason: &'static str) {
        metrics::record_staging_cache_event(&self.context.merkle_tree, event, reason);
    }

    fn record_speculative_event(&self, event: &'static str, reason: &'static str) {
        metrics::record_speculative_event(&self.context.merkle_tree, event, reason);
    }

    fn update_pending_queue_metric(&self) {
        metrics::update_pending_queue_items(&self.context.merkle_tree, self.pending_queue_items);
    }

    pub fn refresh_epoch_context(
        &mut self,
        epoch: u64,
        epoch_phases: EpochPhases,
        output_queue: Pubkey,
    ) {
        self.context.epoch = epoch;
        self.context.epoch_phases = epoch_phases;
        self.context.output_queue = output_queue;
        self.context.input_queue_hint = None;
        self.context.output_queue_hint = None;
        self.speculative.reset(epoch);
    }

    pub fn on_queue_update(&mut self, update: &QueueUpdateMessage) {
        let reason = match update.queue_type {
            QueueType::InputStateV2 => {
                self.context.input_queue_hint = Some(update.queue_size);
                self.pending_nullify_items = update.queue_size.min(usize::MAX as u64) as usize;
                "input"
            }
            QueueType::OutputStateV2 => {
                self.context.output_queue_hint = Some(update.queue_size);
                self.pending_append_items = update.queue_size.min(usize::MAX as u64) as usize;
                "output"
            }
            QueueType::AddressV2 => "address",
            _ => "other",
        };
        self.pending_queue_items = self
            .pending_append_items
            .saturating_add(self.pending_nullify_items);
        self.update_pending_queue_metric();
        self.speculative.record(update.clone());
        self.record_cache_event("queue_update", reason);
    }

    fn consume_processed_items(&mut self, append_processed: usize, nullify_processed: usize) {
        if append_processed > 0 {
            let before = self.pending_append_items;
            self.pending_append_items = self.pending_append_items.saturating_sub(append_processed);
            debug!(
                "Pending append items adjusted: {} -> {} (processed {})",
                before, self.pending_append_items, append_processed
            );
        }
        if nullify_processed > 0 {
            let before = self.pending_nullify_items;
            self.pending_nullify_items =
                self.pending_nullify_items.saturating_sub(nullify_processed);
            debug!(
                "Pending nullify items adjusted: {} -> {} (processed {})",
                before, self.pending_nullify_items, nullify_processed
            );
        }
        let before_total = self.pending_queue_items;
        self.pending_queue_items = self
            .pending_append_items
            .saturating_add(self.pending_nullify_items);
        if before_total != self.pending_queue_items {
            debug!(
                "Total pending queue items adjusted: {} -> {}",
                before_total, self.pending_queue_items
            );
        }
        self.update_pending_queue_metric();
        if self.pending_queue_items == 0 {
            self.record_cache_event("queue_drain", "processed");
        }
    }

    pub fn should_prepare_speculative_job(
        &self,
        seconds_until_slot_start: f64,
        backlog: Option<(QueueType, usize)>,
    ) -> bool {
        if self.speculative.is_busy() {
            trace!(
                "Speculation already inflight/job ready for tree {}",
                self.context.merkle_tree
            );
            self.record_speculative_event("skipped", "inflight");
            return false;
        }

        let lead_time_secs = self.context.speculative_lead_time.as_secs_f64();
        let time_ready = seconds_until_slot_start <= lead_time_secs;

        let queue_ready = backlog
            .map(|(queue_type, queue_len)| {
                let threshold = match queue_type {
                    QueueType::InputStateV2 => self.context.speculative_min_nullify_queue_items,
                    QueueType::OutputStateV2 => self.context.speculative_min_append_queue_items,
                    _ => self.context.speculative_min_append_queue_items,
                };
                threshold == 0 || queue_len >= threshold
            })
            .unwrap_or(false);

        let allowed = time_ready || queue_ready;
        if !allowed {
            self.record_speculative_event("skipped", "insufficient_backlog");
        }

        allowed
    }

    pub fn backlog_snapshot(&self) -> Option<(QueueType, usize)> {
        if self.pending_append_items >= self.pending_nullify_items {
            if self.pending_append_items > 0 {
                return Some((QueueType::OutputStateV2, self.pending_append_items));
            }
            if self.pending_nullify_items > 0 {
                return Some((QueueType::InputStateV2, self.pending_nullify_items));
            }
        } else if self.pending_nullify_items > 0 {
            return Some((QueueType::InputStateV2, self.pending_nullify_items));
        }
        None
    }

    pub async fn new(context: BatchContext<R>, initial_root: [u8; 32]) -> Self {
        let key = context.merkle_tree;

        let shared_state =
            get_or_create_shared_state(&PERSISTENT_TREE_STATES, key, initial_root).await;

        let cached_staging = {
            let staging_cache = PERSISTENT_STAGING_TREES.lock().await;
            staging_cache.get(&context.merkle_tree).cloned()
        };

        let initial_epoch = context.epoch;
        Self {
            shared_state,
            context,
            current_light_slot: None,
            cached_staging,
            pending_queue_items: 0,
            pending_append_items: 0,
            pending_nullify_items: 0,
            speculative: SpeculativeEngine::new(initial_epoch),
        }
    }

    fn calculate_light_slot(&self) -> u64 {
        let estimated_slot = self.context.slot_tracker.estimated_current_slot();
        let active_phase_start = self.context.epoch_phases.active.start;
        (estimated_slot - active_phase_start) / self.context.slot_length
    }

    pub async fn process(&mut self) -> Result<usize> {
        let mut total_items_processed = 0;
        let mut retries = 0;

        loop {
            let light_slot = self.calculate_light_slot();
            if let Some(cached_slot) = self.current_light_slot {
                if light_slot != cached_slot {
                    self.current_light_slot = Some(light_slot);
                }
            } else {
                self.current_light_slot = Some(light_slot);
            }

            let iteration_inputs = self.prepare_iteration_inputs().await?;
            let (num_append_batches, num_nullify_batches, queue_result) =
                if let Some(inputs) = iteration_inputs {
                    inputs
                } else {
                    break;
                };

            if num_append_batches == 0 && num_nullify_batches == 0 {
                break;
            }

            match self
                .process_single_iteration_pipelined_with_cache(
                    num_append_batches,
                    num_nullify_batches,
                    queue_result,
                )
                .await
            {
                Ok(items_this_iteration) => {
                    total_items_processed += items_this_iteration;
                    retries = 0;
                }
                Err(e) => {
                    if let Some(coord_err) = e.downcast_ref::<CoordinatorError>() {
                        if coord_err.is_retryable() {
                            retries += 1;

                            if retries >= MAX_COORDINATOR_RETRIES {
                                warn!(
                                    "Max retries ({}) reached for error: {}",
                                    MAX_COORDINATOR_RETRIES, coord_err
                                );
                                self.record_cache_event("invalidate", "max_retries");
                                self.cached_staging = None;
                                break;
                            }

                            if coord_err.requires_resync() {
                                debug!(
                                    "Invalidating cache and resyncing due to: {} (retry {}/{})",
                                    coord_err, retries, MAX_COORDINATOR_RETRIES
                                );
                                self.record_cache_event("invalidate", "retryable_resync");
                                self.cached_staging = None;
                            } else {
                                debug!("Retrying without resync for: {}", coord_err);
                            }

                            continue;
                        }
                    }
                    self.record_cache_event("invalidate", "non_retryable_error");
                    self.cached_staging = None;
                    {
                        let mut staging_cache = PERSISTENT_STAGING_TREES.lock().await;
                        staging_cache.remove(&self.context.merkle_tree);
                    }
                    return Err(e);
                }
            }
        }

        Ok(total_items_processed)
    }

    /// Invalidate the cached staging tree.
    /// Call this when transaction submission fails to ensure the next iteration starts fresh.
    pub async fn invalidate_cache(&mut self) {
        if self.cached_staging.is_some() {
            self.record_cache_event("invalidate", "external_request");
            self.cached_staging = None;
            let mut staging_cache = PERSISTENT_STAGING_TREES.lock().await;
            staging_cache.remove(&self.context.merkle_tree);
        }
    }

    async fn process_single_iteration_pipelined_with_cache(
        &mut self,
        num_append_batches: usize,
        num_nullify_batches: usize,
        queue_result: QueueFetchResult,
    ) -> Result<usize> {
        let iteration_start = Instant::now();
        let phase1_start = Instant::now();

        let pattern = Self::create_interleaving_pattern(num_append_batches, num_nullify_batches);

        let (prep_tx, prep_rx) = tokio::sync::mpsc::channel(PROOF_CHANNEL_BUFFER_SIZE);
        let (proof_tx, proof_rx) = tokio::sync::mpsc::channel(PROOF_CHANNEL_BUFFER_SIZE);

        let proof_config = ProofConfig {
            append_url: self.context.prover_append_url.clone(),
            update_url: self.context.prover_update_url.clone(),
            address_append_url: String::new(),
            polling_interval: self.context.prover_polling_interval,
            max_wait_time: self.context.prover_max_wait_time,
            api_key: self.context.prover_api_key.clone(),
        };

        let proof_gen_handle = tokio::spawn(async move {
            Self::generate_proofs_streaming(prep_rx, proof_tx, proof_config).await
        });

        let append_zkp_batch_size = queue_result
            .append_data
            .as_ref()
            .map(|data| data.zkp_batch_size)
            .unwrap_or(0);
        let nullify_zkp_batch_size = queue_result
            .nullify_data
            .as_ref()
            .map(|data| data.zkp_batch_size)
            .unwrap_or(0);

        let (staging_after, final_root) = self
            .prepare_batches_streaming(
                queue_result.staging,
                queue_result.output_queue_v2.as_ref(),
                queue_result.input_queue_v2.as_ref(),
                queue_result.on_chain_root,
                &pattern,
                queue_result.append_data.as_ref(),
                queue_result.nullify_data.as_ref(),
                prep_tx,
            )
            .await?;
        self.cache_staging_tree(staging_after.clone(), final_root)
            .await;

        {
            let mut state = self.shared_state.write().await;
            state.update_root(final_root);
        }

        let phase1_duration = phase1_start.elapsed();

        let (total_items, phase3_duration) = self
            .submit_proofs_streaming_inline(
                proof_rx,
                &pattern,
                append_zkp_batch_size,
                nullify_zkp_batch_size,
            )
            .await?;

        let phase2_duration = proof_gen_handle.await??;
        let total_duration = iteration_start.elapsed();

        let append_processed_items =
            num_append_batches.saturating_mul(append_zkp_batch_size as usize);
        let nullify_processed_items =
            num_nullify_batches.saturating_mul(nullify_zkp_batch_size as usize);

        let telemetry = IterationTelemetry {
            tree: self.context.merkle_tree,
            prepare_duration: phase1_duration,
            prove_duration: phase2_duration,
            submit_duration: phase3_duration,
            total_duration,
            append_batches: num_append_batches,
            nullify_batches: num_nullify_batches,
            items_processed: append_processed_items + nullify_processed_items,
        };
        telemetry.report();

        self.mark_batches_processed(
            queue_result.append_batch_ids,
            queue_result.nullify_batch_ids,
        )
        .await;

        let current_root = self.get_current_onchain_root().await?;
        validate_root(current_root, final_root, "pipelined execution")?;

        self.consume_processed_items(append_processed_items, nullify_processed_items);

        Ok(total_items)
    }

    async fn cache_staging_tree(
        &mut self,
        staging: super::tree_state::StagingTree,
        final_root: [u8; 32],
    ) {
        self.cached_staging = Some((staging.clone(), final_root));
        {
            let mut staging_cache = PERSISTENT_STAGING_TREES.lock().await;
            staging_cache.insert(self.context.merkle_tree, (staging, final_root));
        }
        self.record_cache_event("store", "post_iteration");
    }

    async fn prepare_iteration_inputs(
        &mut self,
    ) -> Result<Option<(usize, usize, QueueFetchResult)>> {
        if self.cached_staging.is_none() {
            self.record_cache_event("miss", "not_present");
        }

        let rpc = self.context.rpc_pool.get_connection().await?;
        let mut accounts = rpc
            .get_multiple_accounts(&[self.context.merkle_tree, self.context.output_queue])
            .await?;

        let mut merkle_tree_account = accounts[0]
            .take()
            .ok_or_else(|| anyhow::anyhow!("Merkle tree account not found"))?;
        let output_queue_account = accounts[1].take();
        drop(rpc);

        self.sync_with_chain_with_accounts(&mut merkle_tree_account, output_queue_account.as_ref())
            .await?;

        let tree_data = BatchedMerkleTreeAccount::state_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &self.context.merkle_tree.into(),
        )?;
        let on_chain_root = tree_data
            .root_history
            .last()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("No root in tree history"))?;

        if let Some((_, cached_root)) = &self.cached_staging {
            if *cached_root != on_chain_root {
                info!(
                    "Cache is STALE for speculation prep: cached_root={:?}, on_chain_root={:?}",
                    &cached_root[..8],
                    &on_chain_root[..8]
                );
                self.cached_staging = None;
                let mut staging_cache = PERSISTENT_STAGING_TREES.lock().await;
                staging_cache.remove(&self.context.merkle_tree);
                self.record_cache_event("invalidate", "root_mismatch");
            }
        }

        let (num_append_batches, num_nullify_batches) = self
            .check_readiness_with_accounts(&mut merkle_tree_account, output_queue_account.as_ref())
            .await?;

        if num_append_batches == 0 && num_nullify_batches == 0 {
            return Ok(None);
        }

        let queue_result = self
            .fetch_queues_with_accounts(
                num_append_batches,
                num_nullify_batches,
                merkle_tree_account,
                output_queue_account,
            )
            .await?;

        Ok(Some((
            num_append_batches,
            num_nullify_batches,
            queue_result,
        )))
    }

    pub async fn prepare_speculative_job(&mut self) -> Result<Option<usize>> {
        if !self.speculative.mark_inflight() {
            self.record_speculative_event("skipped", "inflight");
            return Ok(None);
        }

        let job_result = async {
            let (num_append_batches, num_nullify_batches, queue_result) =
                match self.prepare_iteration_inputs().await? {
                    Some(data) => data,
                    None => return Ok(None),
                };

            let pattern =
                Self::create_interleaving_pattern(num_append_batches, num_nullify_batches);

            let _iteration_start = Instant::now();
            let phase1_start = Instant::now();

            let (prep_tx, prep_rx) = tokio::sync::mpsc::channel(50);
            let (proof_tx, proof_rx) = tokio::sync::mpsc::channel(50);

            let proof_config = ProofConfig {
                append_url: self.context.prover_append_url.clone(),
                update_url: self.context.prover_update_url.clone(),
                address_append_url: String::new(),
                polling_interval: self.context.prover_polling_interval,
                max_wait_time: self.context.prover_max_wait_time,
                api_key: self.context.prover_api_key.clone(),
            };

            let proof_gen_handle = tokio::spawn(async move {
                Self::generate_proofs_streaming(prep_rx, proof_tx, proof_config).await
            });

            let (staging_after, final_root) = self
                .prepare_batches_streaming(
                    queue_result.staging,
                    queue_result.output_queue_v2.as_ref(),
                    queue_result.input_queue_v2.as_ref(),
                    queue_result.on_chain_root,
                    &pattern,
                    queue_result.append_data.as_ref(),
                    queue_result.nullify_data.as_ref(),
                    prep_tx,
                )
                .await?;

            let phase1_duration = phase1_start.elapsed();

            let (append_proofs, nullify_proofs, total_items) =
                Self::collect_proofs_streaming(proof_rx).await?;

            let phase2_duration = proof_gen_handle.await??;
            let job = SpeculativeJob {
                pattern,
                append_proofs,
                nullify_proofs,
                append_zkp_batch_size: queue_result
                    .append_data
                    .as_ref()
                    .map(|d| d.zkp_batch_size)
                    .unwrap_or(0),
                nullify_zkp_batch_size: queue_result
                    .nullify_data
                    .as_ref()
                    .map(|d| d.zkp_batch_size)
                    .unwrap_or(0),
                append_batch_ids: queue_result.append_batch_ids,
                nullify_batch_ids: queue_result.nullify_batch_ids,
                staging: staging_after,
                final_root,
                total_items,
                phase1_duration,
                phase2_duration,
                append_batches: num_append_batches,
                nullify_batches: num_nullify_batches,
            };

            Ok(Some((job, total_items)))
        }
        .await;

        match job_result {
            Ok(Some((job, total_items))) => {
                self.speculative.store_job(job);
                info!(
                    "Speculative job prepared for tree {}",
                    self.context.merkle_tree
                );
                self.record_speculative_event("prepared", "ok");
                Ok(Some(total_items))
            }
            Ok(None) => {
                self.speculative.clear_inflight();
                self.record_speculative_event("skipped", "not_ready");
                Ok(None)
            }
            Err(e) => {
                self.speculative.clear_inflight();
                self.record_speculative_event("failed", "prepare_error");
                Err(e)
            }
        }
    }

    pub async fn try_execute_speculative_job(&mut self) -> Result<Option<usize>> {
        if let Some(job) = self.speculative.take_job() {
            info!(
                "Submitting speculative job for tree {} ({} items)",
                self.context.merkle_tree, job.total_items
            );
            let submit_start = Instant::now();
            let submission_result = batch_submission::submit_interleaved_batches(
                &self.context,
                job.append_proofs,
                job.append_zkp_batch_size,
                job.nullify_proofs,
                job.nullify_zkp_batch_size,
                &job.pattern,
            )
            .await;
            let total_items = match submission_result {
                Ok(value) => value,
                Err(e) => {
                    self.record_speculative_event("failed", "submit_error");
                    return Err(e);
                }
            };
            let phase3_duration = submit_start.elapsed();

            let append_processed_items =
                job.append_batches.saturating_mul(job.append_zkp_batch_size as usize);
            let nullify_processed_items =
                job.nullify_batches.saturating_mul(job.nullify_zkp_batch_size as usize);

            // Report telemetry
            let telemetry = IterationTelemetry {
                tree: self.context.merkle_tree,
                prepare_duration: job.phase1_duration,
                prove_duration: job.phase2_duration,
                submit_duration: phase3_duration,
                total_duration: job.phase1_duration + job.phase2_duration + phase3_duration,
                append_batches: job.append_batches,
                nullify_batches: job.nullify_batches,
                items_processed: append_processed_items + nullify_processed_items,
            };
            telemetry.report();

            self.cache_staging_tree(job.staging.clone(), job.final_root)
                .await;

            {
                let mut state = self.shared_state.write().await;
                state.update_root(job.final_root);
            }

            self.mark_batches_processed(job.append_batch_ids, job.nullify_batch_ids)
                .await;

            let current_root = self.get_current_onchain_root().await?;
            if let Err(e) = validate_root(current_root, job.final_root, "speculative submission") {
                self.record_speculative_event("failed", "root_validation");
                return Err(e);
            }

            self.consume_processed_items(append_processed_items, nullify_processed_items);
            self.record_speculative_event("executed", "ok");

            Ok(Some(total_items))
        } else {
            Ok(None)
        }
    }

    async fn check_readiness_with_accounts(
        &self,
        merkle_tree_account: &mut Account,
        output_queue_account: Option<&Account>,
    ) -> Result<(usize, usize)> {
        if let (Some(0), Some(0)) = (
            self.context.input_queue_hint,
            self.context.output_queue_hint,
        ) {
            return Ok((0, 0));
        }

        let tree_data = BatchedMerkleTreeAccount::state_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &self.context.merkle_tree.into(),
        )?;

        let shared_state = self.shared_state.read().await;
        let processed_batches = &shared_state.processed_batches;

        let num_nullify_batches = batch_utils::count_ready_batches(
            &tree_data.queue_batches.batches,
            processed_batches,
            false, // is_append
            false, // calculate_start_index (not needed for state trees)
        );

        let num_append_batches = if let Some(queue_account) = output_queue_account {
            let mut queue_account_data = queue_account.data.clone();
            let queue_data = BatchedQueueAccount::output_from_bytes(&mut queue_account_data)?;
            batch_utils::count_ready_batches(
                &queue_data.batch_metadata.batches,
                processed_batches,
                true,  // is_append
                false, // calculate_start_index (not needed for state trees)
            )
        } else {
            0
        };

        drop(shared_state);
        Ok((num_append_batches, num_nullify_batches))
    }

    async fn parse_onchain_accounts(
        &self,
        num_append_batches: usize,
        num_nullify_batches: usize,
        mut merkle_tree_account: Account,
        output_queue_account: Option<Account>,
    ) -> Result<OnchainState> {
        let merkle_tree_parsed = BatchedMerkleTreeAccount::state_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &self.context.merkle_tree.into(),
        )?;

        let current_onchain_root = merkle_tree_parsed
            .root_history
            .last()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("No root in tree history"))?;

        let (append_metadata, nullify_metadata, append_batch_ids, nullify_batch_ids) =
            if num_append_batches > 0 {
                let mut queue_account = output_queue_account
                    .ok_or_else(|| anyhow::anyhow!("Output queue account not found"))?;

                let output_queue_parsed =
                    BatchedQueueAccount::output_from_bytes(queue_account.data.as_mut_slice())?;

                let (tree_data, queue_data, nullify_ids, append_ids) = self
                    .parse_tree_and_queue_data(
                        &merkle_tree_parsed,
                        &output_queue_parsed,
                        num_nullify_batches > 0,
                        true,
                    )
                    .await?;

                (
                    Some((tree_data.clone(), queue_data)),
                    Some(tree_data),
                    append_ids,
                    nullify_ids,
                )
            } else {
                let (tree_data, nullify_ids) =
                    self.parse_tree_data(&merkle_tree_parsed, true).await?;
                (None, Some(tree_data), vec![], nullify_ids)
            };

        Ok(OnchainState {
            current_onchain_root,
            append_metadata,
            nullify_metadata,
            append_batch_ids,
            nullify_batch_ids,
        })
    }

    async fn fetch_indexed_queues(
        &self,
        parsed_state: &OnchainState,
        num_nullify_batches: usize,
    ) -> Result<light_client::indexer::QueueElementsV2Result> {
        let (output_queue_limit, input_queue_limit) = Self::calculate_queue_limits(
            &parsed_state.append_metadata,
            &parsed_state.nullify_metadata,
            num_nullify_batches,
        );

        // Don't use start_index filtering - we need all elements from index 0 to properly reconstruct the tree.
        // The `limit` parameter (calculated from current_zkp_batch_index) already ensures we get the right range.
        let output_queue_start_index = None;
        let input_queue_start_index = None;

        let mut connection = self.context.rpc_pool.get_connection().await?;
        let indexer = connection.indexer_mut()?;

        let options = light_client::indexer::QueueElementsV2Options {
            output_queue_start_index,
            output_queue_limit,
            input_queue_start_index,
            input_queue_limit,
            address_queue_start_index: None,
            address_queue_limit: None,
            address_queue_zkp_batch_size: None,
        };

        info!(
            "Photon request: tree={}, output_queue_limit={:?}, input_queue_limit={:?}",
            self.context.merkle_tree, output_queue_limit, input_queue_limit
        );

        let queue_elements_response = indexer
            .get_queue_elements_v2(self.context.merkle_tree.to_bytes(), options, None)
            .await?;

        info!(
            "Photon response: output_queue={} elements (initial_root={:?}), input_queue={} elements (initial_root={:?})",
            queue_elements_response.value.output_queue.as_ref().map(|q| q.leaf_indices.len()).unwrap_or(0),
            queue_elements_response.value.output_queue.as_ref().map(|q| {
                let bytes: &[u8] = q.initial_root.as_ref();
                &bytes[..8.min(bytes.len())]
            }),
            queue_elements_response.value.input_queue.as_ref().map(|q| q.leaf_indices.len()).unwrap_or(0),
            queue_elements_response.value.input_queue.as_ref().map(|q| {
                let bytes: &[u8] = q.initial_root.as_ref();
                &bytes[..8.min(bytes.len())]
            })
        );

        drop(connection);

        Ok(queue_elements_response.value)
    }

    fn construct_result(
        &self,
        parsed_state: OnchainState,
        output_queue_v2: Option<light_client::indexer::OutputQueueDataV2>,
        input_queue_v2: Option<light_client::indexer::InputQueueDataV2>,
    ) -> Result<QueueFetchResult> {
        let staging = super::tree_state::StagingTree::from_v2_response(
            output_queue_v2.as_ref(),
            input_queue_v2.as_ref(),
        )?;

        let append_data = Self::build_append_data(
            output_queue_v2.as_ref(),
            &parsed_state.append_metadata,
            &self.context,
        )?;
        let nullify_data = Self::build_nullify_data(
            input_queue_v2.as_ref(),
            &parsed_state.nullify_metadata,
            &self.context,
        )?;

        Ok(QueueFetchResult {
            staging,
            output_queue_v2,
            input_queue_v2,
            on_chain_root: parsed_state.current_onchain_root,
            append_data,
            nullify_data,
            append_batch_ids: parsed_state.append_batch_ids,
            nullify_batch_ids: parsed_state.nullify_batch_ids,
        })
    }

    async fn fetch_queues_with_accounts(
        &self,
        num_append_batches: usize,
        num_nullify_batches: usize,
        merkle_tree_account: Account,
        output_queue_account: Option<Account>,
    ) -> Result<QueueFetchResult> {
        let connection = self.context.rpc_pool.get_connection().await?;
        forester_utils::utils::wait_for_indexer(&*connection)
            .await
            .map_err(|e| anyhow::anyhow!("Indexer failed to catch up before fetch: {}", e))?;
        drop(connection);

        let parsed_state = self
            .parse_onchain_accounts(
                num_append_batches,
                num_nullify_batches,
                merkle_tree_account,
                output_queue_account,
            )
            .await?;

        let indexed_queues = self
            .fetch_indexed_queues(&parsed_state, num_nullify_batches)
            .await?;

        let (output_queue_v2, input_queue_v2) = Self::validate_queue_responses(
            indexed_queues,
            parsed_state.current_onchain_root,
            &parsed_state.append_metadata,
            &parsed_state.nullify_metadata,
        )?;

        self.construct_result(parsed_state, output_queue_v2, input_queue_v2)
    }

    fn calculate_queue_limits(
        append_metadata: &Option<(ParsedMerkleTreeData, ParsedQueueData)>,
        nullify_metadata: &Option<ParsedMerkleTreeData>,
        num_nullify_batches: usize,
    ) -> (Option<u16>, Option<u16>) {
        // Use current_zkp_batch_index to fetch all elements needed for tree reconstruction,
        // not just unprocessed ones. StagingTree needs all batch elements to compute correct intermediate nodes.
        let output_queue_limit = append_metadata.as_ref().map(|(_, queue_data)| {
            let limit = (queue_data.current_zkp_batch_index * queue_data.zkp_batch_size as u64) as u16;
            info!(
                "Output queue limit calculation: current_zkp_batch_index={}, zkp_batch_size={}, limit={}",
                queue_data.current_zkp_batch_index, queue_data.zkp_batch_size, limit
            );
            limit
        });

        let input_queue_limit = if num_nullify_batches > 0 {
            nullify_metadata.as_ref().map(|tree_data| {
                let limit = (tree_data.current_zkp_batch_index * tree_data.zkp_batch_size as u64) as u16;
                info!(
                    "Input queue limit calculation: current_zkp_batch_index={}, zkp_batch_size={}, limit={}, num_nullify_batches={}",
                    tree_data.current_zkp_batch_index, tree_data.zkp_batch_size, limit, num_nullify_batches
                );
                limit
            })
        } else {
            None
        };

        (output_queue_limit, input_queue_limit)
    }

    fn validate_queue_responses(
        response: light_client::indexer::QueueElementsV2Result,
        current_onchain_root: [u8; 32],
        append_metadata: &Option<(ParsedMerkleTreeData, ParsedQueueData)>,
        nullify_metadata: &Option<ParsedMerkleTreeData>,
    ) -> Result<(
        Option<light_client::indexer::OutputQueueDataV2>,
        Option<light_client::indexer::InputQueueDataV2>,
    )> {
        let output_queue = if let Some(ref metadata) = append_metadata {
            if let Some(oq) = response.output_queue {
                batch_utils::validate_photon_root(oq.initial_root, current_onchain_root, "output")?;

                let (_, queue_data) = metadata;
                let expected_total =
                    queue_data.zkp_batch_size as usize * queue_data.leaves_hash_chains.len();
                if oq.leaf_indices.len() != expected_total {
                    anyhow::bail!(
                        "Expected {} output elements, got {}",
                        expected_total,
                        oq.leaf_indices.len()
                    );
                }

                Some(oq)
            } else {
                None
            }
        } else {
            None
        };

        let input_queue = if nullify_metadata.is_some() {
            if let Some(iq) = response.input_queue {
                batch_utils::validate_photon_root(iq.initial_root, current_onchain_root, "input")?;

                let tree_data = nullify_metadata.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("nullify_metadata unexpectedly None despite being checked")
                })?;
                let expected_total =
                    tree_data.zkp_batch_size as usize * tree_data.leaves_hash_chains.len();
                if iq.leaf_indices.len() != expected_total {
                    anyhow::bail!(
                        "Expected {} input elements, got {}",
                        expected_total,
                        iq.leaf_indices.len()
                    );
                }

                Some(iq)
            } else {
                None
            }
        } else {
            None
        };

        Ok((output_queue, input_queue))
    }

    fn build_append_data(
        output_queue: Option<&light_client::indexer::OutputQueueDataV2>,
        metadata: &Option<(ParsedMerkleTreeData, ParsedQueueData)>,
        context: &BatchContext<R>,
    ) -> Result<Option<AppendQueueData>> {
        if let Some(oq) = output_queue {
            let (_, queue_data) = metadata.as_ref().ok_or_else(|| {
                anyhow::anyhow!("metadata unexpectedly None when output_queue is Some")
            })?;

            let queue_elements = oq
                .leaf_indices
                .iter()
                .zip(oq.account_hashes.iter())
                .zip(oq.old_leaves.iter())
                .map(|((&leaf_index, &account_hash), &old_leaf)| {
                    light_client::indexer::MerkleProofWithContext {
                        proof: vec![],
                        root: oq.initial_root,
                        leaf_index,
                        leaf: old_leaf,
                        merkle_tree: context.merkle_tree.to_bytes(),
                        root_seq: 0,
                        tx_hash: None,
                        account_hash,
                    }
                })
                .collect();

            Ok(Some(AppendQueueData {
                queue_elements,
                leaves_hash_chains: queue_data.leaves_hash_chains.clone(),
                zkp_batch_size: queue_data.zkp_batch_size,
            }))
        } else {
            Ok(None)
        }
    }

    fn build_nullify_data(
        input_queue: Option<&light_client::indexer::InputQueueDataV2>,
        metadata: &Option<ParsedMerkleTreeData>,
        context: &BatchContext<R>,
    ) -> Result<Option<NullifyQueueData>> {
        if let Some(iq) = input_queue {
            let tree_data = metadata.as_ref().ok_or_else(|| {
                anyhow::anyhow!("metadata unexpectedly None when input_queue is Some")
            })?;

            let queue_elements = iq
                .leaf_indices
                .iter()
                .zip(iq.account_hashes.iter())
                .zip(iq.current_leaves.iter())
                .zip(iq.tx_hashes.iter())
                .map(
                    |(((&leaf_index, &account_hash), &current_leaf), &tx_hash)| {
                        light_client::indexer::MerkleProofWithContext {
                            proof: vec![],
                            root: iq.initial_root,
                            leaf_index,
                            leaf: current_leaf,
                            merkle_tree: context.merkle_tree.to_bytes(),
                            root_seq: 0,
                            tx_hash: Some(tx_hash),
                            account_hash,
                        }
                    },
                )
                .collect();

            Ok(Some(NullifyQueueData {
                queue_elements,
                leaves_hash_chains: tree_data.leaves_hash_chains.clone(),
                zkp_batch_size: tree_data.zkp_batch_size,
                num_inserted_zkps: tree_data.num_inserted_zkps,
            }))
        } else {
            Ok(None)
        }
    }

    async fn prepare_batches_streaming(
        &mut self,
        staging: super::tree_state::StagingTree,
        output_queue_v2: Option<&light_client::indexer::OutputQueueDataV2>,
        input_queue_v2: Option<&light_client::indexer::InputQueueDataV2>,
        on_chain_root: [u8; 32],
        pattern: &[BatchType],
        append_data: Option<&AppendQueueData>,
        nullify_data: Option<&NullifyQueueData>,
        tx: tokio::sync::mpsc::Sender<(usize, PreparedBatch)>,
    ) -> Result<(super::tree_state::StagingTree, [u8; 32])> {
        let append_leaf_indices: Vec<u64> = if let Some(data) = append_data {
            data.queue_elements
                .iter()
                .map(|elem| elem.leaf_index)
                .collect()
        } else {
            vec![]
        };

        let mut state = if let Some((cached_staging, cached_root)) = self.cached_staging.take() {
            if cached_root == on_chain_root {
                self.record_cache_event("hit", "prep_root_match");

                PreparationState::with_cached_staging(
                    append_leaf_indices,
                    cached_staging,
                    output_queue_v2,
                    input_queue_v2,
                    on_chain_root,
                )
            } else {
                self.record_cache_event("invalidate", "prep_root_mismatch");
                PreparationState::new(staging, append_leaf_indices)
            }
        } else {
            PreparationState::new(staging, append_leaf_indices)
        };
        let mut prepared_batches = Vec::with_capacity(pattern.len());

        for (i, batch_type) in pattern.iter().enumerate() {
            let prepared_batch = match batch_type {
                BatchType::Append => {
                    let append_data =
                        append_data.ok_or_else(|| anyhow::anyhow!("Append data not available"))?;
                    let circuit_inputs =
                        batch_preparation::prepare_append_batch(append_data, &mut state)?;

                    if i < 3 {
                        let old_root_bytes = circuit_inputs.old_root.to_bytes_be().1;
                        let new_root = state.staging.current_root();
                        debug!(
                            "Prepared append batch {} at position {}: start_index={}, batch_size={}, old_root={:?}, new_root={:?}",
                            state.append_batch_index - 1,
                            i,
                            circuit_inputs.start_index,
                            circuit_inputs.batch_size,
                            &old_root_bytes[old_root_bytes.len().saturating_sub(8)..],
                            &new_root[..8]
                        );
                    }

                    PreparedBatch::Append(circuit_inputs)
                }
                BatchType::Nullify => {
                    let nullify_data = nullify_data
                        .ok_or_else(|| anyhow::anyhow!("Nullify data not available"))?;
                    let circuit_inputs =
                        batch_preparation::prepare_nullify_batch(nullify_data, &mut state)?;

                    if i < 3 {
                        let old_root_bytes = circuit_inputs.old_root.to_bytes_be().1;
                        let new_root = state.staging.current_root();
                        debug!(
                            "Prepared nullify batch {} at position {}: path_indices={:?}, old_root={:?}, new_root={:?}",
                            state.nullify_batch_index - 1,
                            i,
                            &circuit_inputs.path_indices,
                            &old_root_bytes[old_root_bytes.len().saturating_sub(8)..],
                            &new_root[..8]
                        );
                    }

                    PreparedBatch::Nullify(circuit_inputs)
                }
                BatchType::Address => {
                    return Err(anyhow::anyhow!(
                        "Address batch type not supported in state tree coordinator"
                    ))
                }
            };

            debug!(
                "After preparing batch {} (type={:?}): staging.current_root={:?}",
                i,
                batch_type.as_str(),
                &state.staging.current_root()[..8]
            );

            prepared_batches.push((i, prepared_batch));
        }

        for (i, prepared_batch) in prepared_batches {
            tx.send((i, prepared_batch))
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send prepared batch: {}", e))?;
        }

        let final_root = state.staging.current_root();
        let staging_to_cache = state.staging.clone();

        Ok((staging_to_cache, final_root))
    }

    async fn generate_proofs_streaming(
        mut prep_rx: tokio::sync::mpsc::Receiver<(usize, PreparedBatch)>,
        proof_tx: tokio::sync::mpsc::Sender<(usize, PreparedBatch, Result<ProofResult>)>,
        config: ProofConfig,
    ) -> Result<Duration> {
        let append_client = Arc::new(ProofClient::with_config(
            config.append_url.clone(),
            config.polling_interval,
            config.max_wait_time,
            config.api_key.clone(),
        ));

        let nullify_client = Arc::new(ProofClient::with_config(
            config.update_url.clone(),
            config.polling_interval,
            config.max_wait_time,
            config.api_key.clone(),
        ));

        let mut poll_handles = Vec::new();
        let proof_gen_start = Instant::now();
        let mut job_count = 0;

        while let Some((idx, prepared_batch)) = prep_rx.recv().await {
            let proof_tx_clone = proof_tx.clone();

            match &prepared_batch {
                PreparedBatch::Append(circuit_inputs) => {
                    debug!("Submitting append proof request for batch {}", idx);
                    let inputs_json =
                        BatchAppendInputsJson::from_inputs(circuit_inputs).to_string();
                    match append_client
                        .submit_proof_async(inputs_json.clone(), "append")
                        .await
                    {
                        Ok(job_id) => {
                            info!("Batch {} (append) submitted with job_id: {}", idx, job_id);
                            job_count += 1;

                            let client = append_client.clone();
                            let circuit_inputs = circuit_inputs.clone();
                            let inputs_json_clone = inputs_json.clone();
                            let handle = tokio::spawn(async move {
                                debug!(
                                    "Polling for append proof completion: batch {}, job {}",
                                    idx, job_id
                                );

                                let result = super::proof_pipeline::poll_proof_with_retry(
                                    client,
                                    job_id,
                                    inputs_json_clone,
                                    "append",
                                    idx,
                                    |proof| {
                                        proof_utils::create_append_proof_result(&circuit_inputs, proof)
                                            .map(ProofResult::Append)
                                    },
                                )
                                .await;

                                let _ = proof_tx_clone
                                    .send((idx, PreparedBatch::Append(circuit_inputs), result))
                                    .await;
                            });
                            poll_handles.push(handle);
                        }
                        Err(e) => {
                            error!("Failed to submit batch {} (append): {}", idx, e);
                            let _ = proof_tx_clone
                                .send((idx, prepared_batch, Err(anyhow::anyhow!("{}", e))))
                                .await;
                        }
                    }
                }
                PreparedBatch::Nullify(circuit_inputs) => {
                    debug!("Submitting nullify proof request for batch {}", idx);
                    let inputs_json = update_inputs_string(circuit_inputs);
                    match nullify_client
                        .submit_proof_async(inputs_json.clone(), "update")
                        .await
                    {
                        Ok(job_id) => {
                            info!("Batch {} (nullify) submitted with job_id: {}", idx, job_id);
                            job_count += 1;

                            let client = nullify_client.clone();
                            let circuit_inputs = circuit_inputs.clone();
                            let inputs_json_clone = inputs_json.clone();
                            let handle = tokio::spawn(async move {
                                debug!(
                                    "Polling for nullify proof completion: batch {}, job {}",
                                    idx, job_id
                                );

                                let result = super::proof_pipeline::poll_proof_with_retry(
                                    client,
                                    job_id,
                                    inputs_json_clone,
                                    "update",
                                    idx,
                                    |proof| {
                                        proof_utils::create_nullify_proof_result(&circuit_inputs, proof)
                                            .map(ProofResult::Nullify)
                                    },
                                )
                                .await;

                                let _ = proof_tx_clone
                                    .send((idx, PreparedBatch::Nullify(circuit_inputs), result))
                                    .await;
                            });
                            poll_handles.push(handle);
                        }
                        Err(e) => {
                            error!("Failed to submit batch {} (nullify): {}", idx, e);
                            let _ = proof_tx_clone
                                .send((idx, prepared_batch, Err(anyhow::anyhow!("{}", e))))
                                .await;
                        }
                    }
                }
                PreparedBatch::Address(_) => {
                    error!("Address batch submitted to state tree coordinator - this is a bug");
                    let _ = proof_tx_clone
                        .send((
                            idx,
                            prepared_batch,
                            Err(anyhow::anyhow!(
                                "Address batches should not be processed by state tree coordinator"
                            )),
                        ))
                        .await;
                }
            }
        }

        info!(
            "All {} proof requests submitted and polling started",
            job_count
        );

        drop(proof_tx);

        for handle in poll_handles {
            handle
                .await
                .map_err(|e| anyhow::anyhow!("Proof polling task join error: {}", e))?;
        }

        let proof_gen_duration = proof_gen_start.elapsed();

        info!(
            "Phase 2 complete: All proofs received in {:?}",
            proof_gen_duration
        );

        Ok(proof_gen_duration)
    }

    async fn submit_proofs_streaming_inline(
        &self,
        mut proof_rx: tokio::sync::mpsc::Receiver<(usize, PreparedBatch, Result<ProofResult>)>,
        pattern: &[BatchType],
        append_zkp_batch_size: u16,
        nullify_zkp_batch_size: u16,
    ) -> Result<(usize, Duration)> {
        use std::collections::BTreeMap;

        let mut buffer: BTreeMap<usize, (PreparedBatch, Result<ProofResult>)> = BTreeMap::new();
        let mut next_to_submit = 0;
        let mut total_items = 0;
        let mut total_submit_duration = Duration::ZERO;
        let mut ready_append_proofs = Vec::new();
        let mut ready_nullify_proofs = Vec::new();
        let mut ready_pattern = Vec::new();

        while let Some((idx, batch, proof_result)) = proof_rx.recv().await {
            buffer.insert(idx, (batch, proof_result));

            while let Some((batch, proof_result)) = buffer.remove(&next_to_submit) {
                if matches!(batch, PreparedBatch::Address(_)) {
                    error!("Address batch in state tree coordinator - this is a bug");
                    return Err(anyhow::anyhow!(
                        "Address batches should not be processed by state tree coordinator"
                    ));
                }

                let proof = proof_result.map_err(|e| {
                    let err_msg = e.to_string();
                    // Detect constraint errors which indicate stale tree state
                    if err_msg.contains("constraint #") && err_msg.contains("is not satisfied") {
                        // Log detailed batch information to help diagnose the issue
                        match &batch {
                            PreparedBatch::Append(inputs) => {
                                let old_root_bytes = inputs.old_root.to_bytes_be().1;
                                let new_root_bytes = inputs.new_root.to_bytes_be().1;
                                warn!(
                                    "Constraint error in APPEND batch {}: start_index={}, batch_size={}, old_root={:?}, new_root={:?}",
                                    next_to_submit,
                                    inputs.start_index,
                                    inputs.batch_size,
                                    &old_root_bytes[old_root_bytes.len().saturating_sub(8)..],
                                    &new_root_bytes[new_root_bytes.len().saturating_sub(8)..]
                                );
                            }
                            PreparedBatch::Nullify(inputs) => {
                                let old_root_bytes = inputs.old_root.to_bytes_be().1;
                                let new_root_bytes = inputs.new_root.to_bytes_be().1;
                                warn!(
                                    "Constraint error in NULLIFY batch {}: path_indices={:?}, old_root={:?}, new_root={:?}",
                                    next_to_submit,
                                    &inputs.path_indices,
                                    &old_root_bytes[old_root_bytes.len().saturating_sub(8)..],
                                    &new_root_bytes[new_root_bytes.len().saturating_sub(8)..]
                                );
                            }
                            PreparedBatch::Address(_) => {
                                // Already checked above, unreachable
                            }
                        }
                        warn!(
                            "Constraint error detected in batch {} (likely stale tree state from partial on-chain commit). Will resync and retry.",
                            next_to_submit
                        );
                        CoordinatorError::ConstraintError {
                            batch_index: next_to_submit,
                            details: err_msg,
                        }
                        .into()
                    } else {
                        e
                    }
                })?;

                let batch_type = pattern
                    .get(next_to_submit)
                    .copied()
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Pattern index {} out of bounds (pattern len={})",
                            next_to_submit,
                            pattern.len()
                        )
                    })?;
                ready_pattern.push(batch_type);

                match proof {
                    ProofResult::Append(append_proof) => {
                        ready_append_proofs.push(append_proof);
                    }
                    ProofResult::Nullify(nullify_proof) => {
                        ready_nullify_proofs.push(nullify_proof);
                    }
                }

                next_to_submit += 1;

                if ready_pattern.len() >= MAX_BATCHES_PER_TRANSACTION {
                    let submit_start = Instant::now();
                    total_items += batch_submission::submit_interleaved_batches(
                        &self.context,
                        std::mem::take(&mut ready_append_proofs),
                        append_zkp_batch_size,
                        std::mem::take(&mut ready_nullify_proofs),
                        nullify_zkp_batch_size,
                        &std::mem::take(&mut ready_pattern),
                    )
                    .await?;
                    total_submit_duration += submit_start.elapsed();
                }
            }
        }

        if !ready_pattern.is_empty() {
            let submit_start = Instant::now();
            total_items += batch_submission::submit_interleaved_batches(
                &self.context,
                ready_append_proofs,
                append_zkp_batch_size,
                ready_nullify_proofs,
                nullify_zkp_batch_size,
                &ready_pattern,
            )
            .await?;
            total_submit_duration += submit_start.elapsed();
        }

        if !buffer.is_empty() {
            anyhow::bail!("Pipeline ended with {} unsubmitted batches", buffer.len());
        }

        info!(
            "Phase 3 actual submission time: {:?}",
            total_submit_duration
        );
        Ok((total_items, total_submit_duration))
    }

    async fn collect_proofs_streaming(
        mut proof_rx: tokio::sync::mpsc::Receiver<(usize, PreparedBatch, Result<ProofResult>)>,
    ) -> Result<(
        Vec<InstructionDataBatchAppendInputs>,
        Vec<InstructionDataBatchNullifyInputs>,
        usize,
    )> {
        let mut buffer: BTreeMap<usize, (PreparedBatch, Result<ProofResult>)> = BTreeMap::new();
        let mut next_to_collect = 0;
        let mut append_proofs = Vec::new();
        let mut nullify_proofs = Vec::new();
        let mut total_items = 0usize;

        while let Some((idx, batch, proof_result)) = proof_rx.recv().await {
            buffer.insert(idx, (batch, proof_result));

            while let Some((batch, proof_result)) = buffer.remove(&next_to_collect) {
                let batch_items = match &batch {
                    PreparedBatch::Append(inputs) => inputs.batch_size as usize,
                    PreparedBatch::Nullify(inputs) => inputs.batch_size as usize,
                    PreparedBatch::Address(_) => {
                        error!("Address batch in proof collection - this is a bug");
                        return Err(anyhow::anyhow!(
                            "Address batches should not be processed by state tree coordinator"
                        ));
                    }
                };
                let proof = proof_result.map_err(|e| anyhow::anyhow!(e))?;
                match proof {
                    ProofResult::Append(append_proof) => {
                        append_proofs.push(append_proof);
                    }
                    ProofResult::Nullify(nullify_proof) => {
                        nullify_proofs.push(nullify_proof);
                    }
                }

                total_items += batch_items;
                next_to_collect += 1;
            }
        }

        if !buffer.is_empty() {
            anyhow::bail!(
                "Speculative pipeline ended with {} pending batches",
                buffer.len()
            );
        }

        Ok((append_proofs, nullify_proofs, total_items))
    }

    fn create_interleaving_pattern(num_appends: usize, num_nullifies: usize) -> Vec<BatchType> {
        let mut pattern = Vec::new();

        if num_appends > 0 {
            pattern.extend(vec![BatchType::Append; num_appends]);
        }

        if num_nullifies > 0 {
            pattern.extend(vec![BatchType::Nullify; num_nullifies]);
        }

        pattern
    }

    async fn mark_batches_processed(
        &self,
        append_batch_ids: Vec<ProcessedBatchId>,
        nullify_batch_ids: Vec<ProcessedBatchId>,
    ) {
        let mut shared_state = self.shared_state.write().await;

        for batch_id in append_batch_ids {
            shared_state.mark_batch_processed(batch_id);
        }

        for batch_id in nullify_batch_ids {
            shared_state.mark_batch_processed(batch_id);
        }
    }

    async fn sync_with_chain_with_accounts(
        &mut self,
        merkle_tree_account: &mut Account,
        output_queue_account: Option<&Account>,
    ) -> Result<bool> {
        let (on_chain_root, tree_batches) = super::sync_utils::extract_state_tree_sync_data(
            merkle_tree_account,
            &self.context.merkle_tree,
        )?;

        let output_queue_batches = if let Some(queue_account) = output_queue_account {
            let mut queue_account_data = queue_account.data.clone();
            let queue_data = BatchedQueueAccount::output_from_bytes(&mut queue_account_data)?;
            queue_data.batch_metadata.batches
        } else {
            [light_batched_merkle_tree::batch::Batch::default(); 2]
        };

        super::sync_utils::sync_coordinator_state(
            &self.shared_state,
            on_chain_root,
            &tree_batches,
            &output_queue_batches,
        )
        .await
    }

    async fn get_current_onchain_root(&self) -> Result<[u8; 32]> {
        let rpc = self.context.rpc_pool.get_connection().await?;
        batch_utils::fetch_state_tree_root(&*rpc, self.context.merkle_tree).await
    }

    async fn parse_tree_and_queue_data(
        &self,
        merkle_tree: &BatchedMerkleTreeAccount<'_>,
        output_queue: &BatchedQueueAccount<'_>,
        collect_nullify_ids: bool,
        collect_append_ids: bool,
    ) -> Result<(
        ParsedMerkleTreeData,
        ParsedQueueData,
        Vec<ProcessedBatchId>,
        Vec<ProcessedBatchId>,
    )> {
        let shared_state = self.shared_state.write().await;

        let mut tree_leaves_hash_chains = Vec::new();
        let mut nullify_batch_ids = Vec::new();
        let mut zkp_batch_size = 0u16;
        let mut batch_start_index = 0u64;

        for (batch_idx, batch) in merkle_tree.queue_batches.batches.iter().enumerate() {
            let batch_state = batch.get_state();
            let not_inserted =
                batch_state != light_batched_merkle_tree::batch::BatchState::Inserted;

            if not_inserted {
                let num_inserted = batch.get_num_inserted_zkps();
                let current_index = batch.get_current_zkp_batch_index();

                // Capture start index of first batch with work (for fetching full batch from Photon)
                if tree_leaves_hash_chains.is_empty() {
                    zkp_batch_size = batch.zkp_batch_size as u16;
                    batch_start_index = batch.start_index;
                }

                for i in num_inserted..current_index {
                    // Always collect hash chains for limit calculation (Photon needs total count)
                    tree_leaves_hash_chains
                        .push(merkle_tree.hash_chain_stores[batch_idx][i as usize]);

                    // Only track batch_ids for unprocessed batches (for actual work)
                    let batch_id = ProcessedBatchId {
                        batch_index: batch_idx,
                        zkp_batch_index: i,
                        is_append: false,
                        start_leaf_index: None,
                    };

                    if !shared_state.is_batch_processed(&batch_id) && collect_nullify_ids {
                        nullify_batch_ids.push(batch_id);
                    }
                }
            }
        }

        let current_root = merkle_tree
            .root_history
            .last()
            .ok_or_else(|| anyhow::anyhow!("Merkle tree has no root history"))?;

        let tree_data = ParsedMerkleTreeData {
            next_index: merkle_tree.next_index,
            current_root: *current_root,
            root_history: merkle_tree.root_history.to_vec(),
            zkp_batch_size,
            pending_batch_index: merkle_tree.queue_batches.pending_batch_index as u32,
            num_inserted_zkps: 0,
            current_zkp_batch_index: tree_leaves_hash_chains.len() as u64,
            batch_start_index,
            leaves_hash_chains: tree_leaves_hash_chains,
        };

        let mut queue_leaves_hash_chains = Vec::new();
        let mut append_batch_ids = Vec::new();
        let mut queue_batch_start_index = 0u64;

        for (batch_idx, batch) in output_queue.batch_metadata.batches.iter().enumerate() {
            let batch_state = batch.get_state();
            let not_inserted =
                batch_state != light_batched_merkle_tree::batch::BatchState::Inserted;

            if not_inserted {
                let num_inserted = batch.get_num_inserted_zkps();
                let current_index = batch.get_current_zkp_batch_index();

                // Capture start index of first batch with pending work
                if queue_leaves_hash_chains.is_empty() {
                    queue_batch_start_index = batch.start_index;
                }

                for i in num_inserted..current_index {
                    // Always collect hash chains for limit calculation (Photon needs total count)
                    queue_leaves_hash_chains
                        .push(output_queue.hash_chain_stores[batch_idx][i as usize]);

                    // Only track batch_ids for unprocessed batches (for actual work)
                    let batch_id = ProcessedBatchId {
                        batch_index: batch_idx,
                        zkp_batch_index: i,
                        is_append: true,
                        start_leaf_index: None,
                    };

                    if !shared_state.is_batch_processed(&batch_id) && collect_append_ids {
                        append_batch_ids.push(batch_id);
                    }
                }
            }
        }
        drop(shared_state);

        let queue_data = ParsedQueueData {
            zkp_batch_size: output_queue.batch_metadata.zkp_batch_size as u16,
            pending_batch_index: output_queue.batch_metadata.pending_batch_index as u32,
            num_inserted_zkps: 0,
            current_zkp_batch_index: queue_leaves_hash_chains.len() as u64,
            batch_start_index: queue_batch_start_index,
            leaves_hash_chains: queue_leaves_hash_chains,
        };

        Ok((tree_data, queue_data, nullify_batch_ids, append_batch_ids))
    }

    async fn parse_tree_data(
        &self,
        merkle_tree: &BatchedMerkleTreeAccount<'_>,
        collect_batch_ids: bool,
    ) -> Result<(ParsedMerkleTreeData, Vec<ProcessedBatchId>)> {
        let shared_state = self.shared_state.write().await;

        let mut leaves_hash_chains = Vec::new();
        let mut batch_ids = Vec::new();
        let mut zkp_batch_size = 0u16;
        let mut batch_start_index = 0u64;

        for (batch_idx, batch) in merkle_tree.queue_batches.batches.iter().enumerate() {
            let batch_state = batch.get_state();
            let not_inserted =
                batch_state != light_batched_merkle_tree::batch::BatchState::Inserted;

            if not_inserted {
                let num_inserted = batch.get_num_inserted_zkps();
                let current_index = batch.get_current_zkp_batch_index();

                // Capture start index of first batch with work (for fetching full batch from Photon)
                if leaves_hash_chains.is_empty() {
                    zkp_batch_size = batch.zkp_batch_size as u16;
                    batch_start_index = batch.start_index;
                }

                for i in num_inserted..current_index {
                    // Always collect hash chains for limit calculation (Photon needs total count)
                    leaves_hash_chains
                        .push(merkle_tree.hash_chain_stores[batch_idx][i as usize]);

                    // Only track batch_ids for unprocessed batches (for actual work)
                    let batch_id = ProcessedBatchId {
                        batch_index: batch_idx,
                        zkp_batch_index: i,
                        is_append: false,
                        start_leaf_index: None,
                    };

                    if !shared_state.is_batch_processed(&batch_id) && collect_batch_ids {
                        batch_ids.push(batch_id);
                    }
                }
            }
        }
        drop(shared_state);

        let current_root = merkle_tree
            .root_history
            .last()
            .ok_or_else(|| anyhow::anyhow!("Merkle tree has no root history"))?;

        let current_zkp_batch_index = leaves_hash_chains.len() as u64;
        info!(
            "parse_tree_data: zkp_batch_size={}, leaves_hash_chains.len()={}, current_zkp_batch_index={}",
            zkp_batch_size, leaves_hash_chains.len(), current_zkp_batch_index
        );

        Ok((
            ParsedMerkleTreeData {
                next_index: merkle_tree.next_index,
                current_root: *current_root,
                root_history: merkle_tree.root_history.to_vec(),
                zkp_batch_size,
                pending_batch_index: merkle_tree.queue_batches.pending_batch_index as u32,
                num_inserted_zkps: 0,
                current_zkp_batch_index,
                batch_start_index,
                leaves_hash_chains,
            },
            batch_ids,
        ))
    }
}

// Performance metrics are now available via Prometheus metrics endpoint.
// See telemetry module for IterationTelemetry reporting.

#[derive(Default)]
struct SpeculativeEngine {
    epoch: u64,
    queued_updates: VecDeque<QueueUpdateMessage>,
    inflight: bool,
    job: Option<SpeculativeJob>,
}

impl SpeculativeEngine {
    fn new(epoch: u64) -> Self {
        Self {
            epoch,
            queued_updates: VecDeque::new(),
            inflight: false,
            job: None,
        }
    }

    fn reset(&mut self, epoch: u64) {
        if self.epoch != epoch {
            self.epoch = epoch;
            self.queued_updates.clear();
            self.job = None;
            self.inflight = false;
        }
    }

    fn record(&mut self, update: QueueUpdateMessage) {
        self.queued_updates.push_back(update);
    }

    fn mark_inflight(&mut self) -> bool {
        if self.inflight || self.job.is_some() {
            return false;
        }
        self.inflight = true;
        true
    }

    fn clear_inflight(&mut self) {
        self.inflight = false;
    }

    fn store_job(&mut self, job: SpeculativeJob) {
        self.job = Some(job);
        self.inflight = false;
    }

    fn take_job(&mut self) -> Option<SpeculativeJob> {
        self.job.take()
    }

    fn is_busy(&self) -> bool {
        self.inflight || self.job.is_some()
    }
}

struct SpeculativeJob {
    pattern: Vec<BatchType>,
    append_proofs: Vec<InstructionDataBatchAppendInputs>,
    nullify_proofs: Vec<InstructionDataBatchNullifyInputs>,
    append_zkp_batch_size: u16,
    nullify_zkp_batch_size: u16,
    append_batch_ids: Vec<ProcessedBatchId>,
    nullify_batch_ids: Vec<ProcessedBatchId>,
    staging: super::tree_state::StagingTree,
    final_root: [u8; 32],
    total_items: usize,
    // Timing for phases 1+2 (phase 3 happens during execute_speculative)
    phase1_duration: Duration,
    phase2_duration: Duration,
    append_batches: usize,
    nullify_batches: usize,
}
