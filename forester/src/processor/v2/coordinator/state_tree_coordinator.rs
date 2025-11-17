use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use forester_utils::{ParsedMerkleTreeData, ParsedQueueData};
use light_batched_merkle_tree::{
    merkle_tree::{
        BatchedMerkleTreeAccount, InstructionDataBatchAppendInputs,
        InstructionDataBatchNullifyInputs,
    },
    queue::BatchedQueueAccount,
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_prover_client::proof_client::ProofClient;
use once_cell::sync::Lazy;
use solana_sdk::{account::Account, pubkey::Pubkey};
use tokio::sync::Mutex as TokioMutex;
use tracing::{debug, error, info, warn};

use super::{
    batch_preparation, batch_submission,
    error::CoordinatorError,
    proof_generation::ProofConfig,
    shared_state::{
        create_shared_state, CumulativeMetrics, IterationMetrics, ProcessedBatchId, SharedState,
    },
    types::{AppendQueueData, BatchType, NullifyQueueData, PreparationState, PreparedBatch},
};
use crate::processor::v2::common::BatchContext;

type PersistentTreeStatesCache = Arc<TokioMutex<HashMap<(Pubkey, u64), SharedState>>>;
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

struct ParsedOnchainState {
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
    /// Cached staging tree to preserve across iterations when root hasn't changed.
    /// Stores (staging_tree, root). The staging tree accumulates all updates across iterations.
    /// When reusing, batch indices reset to 0 for new queue data, but the staging tree
    /// already contains all previous updates, ensuring proofs include prior changes.
    cached_staging: Option<(super::tree_state::StagingTree, [u8; 32])>,
}

impl<R: Rpc> StateTreeCoordinator<R> {
    pub async fn new(context: BatchContext<R>, initial_root: [u8; 32]) -> Self {
        let key = (context.merkle_tree, context.epoch);

        let shared_state = {
            let mut states = PERSISTENT_TREE_STATES.lock().await;
            if let Some(state) = states.get(&key) {
                info!(
                    "COORDINATOR REUSE: Found existing SharedState for tree={}, epoch={} (initial_root={:?})",
                    context.merkle_tree,
                    context.epoch,
                    &initial_root[..8]
                );
                state.clone()
            } else {
                info!(
                    "COORDINATOR CREATE: Creating NEW SharedState for tree={}, epoch={} (initial_root={:?})",
                    context.merkle_tree,
                    context.epoch,
                    &initial_root[..8]
                );
                let new_state = create_shared_state(initial_root);
                states.insert(key, new_state.clone());
                new_state
            }
        };

        cleanup_old_epochs(context.merkle_tree, context.epoch).await;

        // DIAGNOSTIC: Log coordinator instance creation with unique identifier
        let coordinator_id = format!(
            "{}@epoch{}",
            &context.merkle_tree.to_string()[..8],
            context.epoch
        );
        info!(
            "StateTreeCoordinator instance created: id={}, tree={}, epoch={}, initial_root={:?}",
            coordinator_id,
            context.merkle_tree,
            context.epoch,
            &initial_root[..8]
        );

        // Load staging tree from global cache (survives across epochs)
        let cached_staging = {
            let staging_cache = PERSISTENT_STAGING_TREES.lock().await;
            staging_cache.get(&context.merkle_tree).cloned()
        };

        if let Some((ref staging, ref cached_root)) = cached_staging {
            info!(
                "Loaded staging tree from previous epoch for tree={}, cached_root={:?}, on_chain_root={:?}, updates={}",
                context.merkle_tree,
                &cached_root[..8],
                &initial_root[..8],
                staging.get_updates().len()
            );
        }

        Self {
            shared_state,
            context,
            current_light_slot: None,
            cached_staging,
        }
    }

    fn calculate_light_slot(&self) -> u64 {
        let estimated_slot = self.context.slot_tracker.estimated_current_slot();
        let active_phase_start = self.context.epoch_phases.active.start;
        (estimated_slot - active_phase_start) / self.context.slot_length
    }

    pub async fn process(&mut self) -> Result<usize> {
        let mut total_items_processed = 0;
        let mut loop_iteration = 0;
        let mut consecutive_retries = 0;
        const MAX_CONSECUTIVE_RETRIES: usize = 10;

        // DIAGNOSTIC: Log process() entry to track coordinator usage
        debug!(
            "StateTreeCoordinator::process() called for tree={}, epoch={}",
            self.context.merkle_tree, self.context.epoch
        );

        loop {
            loop_iteration += 1;

            let light_slot = self.calculate_light_slot();
            if let Some(cached_slot) = self.current_light_slot {
                if light_slot != cached_slot {
                    debug!(
                        "Light slot changed {} -> {} (caches preserved until root changes)",
                        cached_slot, light_slot
                    );
                    self.current_light_slot = Some(light_slot);
                }
            } else {
                self.current_light_slot = Some(light_slot);
            }

            let (num_append_batches, num_nullify_batches) = {
                let rpc = self.context.rpc_pool.get_connection().await?;
                let mut accounts = rpc
                    .get_multiple_accounts(&[self.context.merkle_tree, self.context.output_queue])
                    .await?;

                let mut merkle_tree_account = accounts[0]
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("Merkle tree account not found"))?;
                let output_queue_account = accounts[1].take();
                drop(rpc);

                // Always sync to update processed_batches with on-chain confirmation state
                self.sync_with_chain_with_accounts(
                    &mut merkle_tree_account,
                    output_queue_account.as_ref(),
                )
                .await?;

                // Check if cached tree is stale by comparing with on-chain root
                let tree_data = BatchedMerkleTreeAccount::state_from_bytes(
                    merkle_tree_account.data.as_mut_slice(),
                    &self.context.merkle_tree.into(),
                )?;
                let on_chain_root = tree_data
                    .root_history
                    .last()
                    .copied()
                    .ok_or_else(|| anyhow::anyhow!("No root in tree history"))?;

                let cache_is_stale = if let Some((_, cached_root)) = &self.cached_staging {
                    let stale = *cached_root != on_chain_root;
                    if stale {
                        info!(
                            "Cache is STALE: cached_root={:?}, on_chain_root={:?}",
                            &cached_root[..8],
                            &on_chain_root[..8]
                        );
                    } else {
                        debug!("Cache is fresh: root={:?}", &on_chain_root[..8]);
                    }
                    stale
                } else {
                    debug!("No cache present, on_chain_root={:?}", &on_chain_root[..8]);
                    true // No cache, need to fetch
                };

                // Only invalidate cache if it's actually stale
                // (transaction failed, or another forester processed batches)
                if cache_is_stale {
                    self.cached_staging = None;
                    // Also clear global cache
                    {
                        let mut staging_cache = PERSISTENT_STAGING_TREES.lock().await;
                        staging_cache.remove(&self.context.merkle_tree);
                        info!(
                            "Invalidated both local and global staging cache for tree={}",
                            self.context.merkle_tree
                        );
                    }
                }

                self.check_readiness_with_accounts(
                    &mut merkle_tree_account,
                    output_queue_account.as_ref(),
                )
                .await?
            };

            if num_append_batches == 0 && num_nullify_batches == 0 {
                break;
            }

            match self
                .process_single_iteration_pipelined_with_cache(
                    num_append_batches,
                    num_nullify_batches,
                    loop_iteration,
                )
                .await
            {
                Ok(items_this_iteration) => {
                    total_items_processed += items_this_iteration;
                    consecutive_retries = 0; // Reset retry counter on success
                }
                Err(e) => {
                    if let Some(coord_err) = e.downcast_ref::<CoordinatorError>() {
                        if coord_err.is_retryable() {
                            consecutive_retries += 1;

                            if consecutive_retries >= MAX_CONSECUTIVE_RETRIES {
                                warn!(
                                    "Max consecutive retries ({}) reached for error: {}. Giving up on this batch.",
                                    MAX_CONSECUTIVE_RETRIES, coord_err
                                );
                                self.cached_staging = None;
                                // Break out of retry loop but don't return error - just move to next iteration
                                break;
                            }

                            // Only invalidate cache (force resync) if the error indicates we're out of sync
                            // For other retryable errors (like PhotonStale), keep our optimistic local state
                            if coord_err.requires_resync() {
                                debug!(
                                    "Invalidating cache and resyncing due to: {} (retry {}/{})",
                                    coord_err, consecutive_retries, MAX_CONSECUTIVE_RETRIES
                                );
                                self.cached_staging = None;
                            } else {
                                debug!("Retrying without resync for: {}", coord_err);
                            }

                            if matches!(coord_err, CoordinatorError::PhotonStale { .. }) {
                                debug!("Photon staleness detected, waiting before retry");
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                            }
                            continue;
                        }
                    }
                    // Clear both local and global cache on error
                    self.cached_staging = None;
                    {
                        let mut staging_cache = PERSISTENT_STAGING_TREES.lock().await;
                        staging_cache.remove(&self.context.merkle_tree);
                    }
                    return Err(e);
                }
            }
        }

        {
            let state = self.shared_state.read().await;
            state.print_performance_summary(&format!(
                "Tree: {}, Epoch: {}",
                self.context.merkle_tree, self.context.epoch
            ));
        }

        Ok(total_items_processed)
    }

    /// Invalidate the cached staging tree.
    /// Call this when transaction submission fails to ensure the next iteration starts fresh.
    pub async fn invalidate_cache(&mut self) {
        if self.cached_staging.is_some() {
            debug!(
                "Invalidating cached staging tree due to external failure (e.g., transaction submission failed)"
            );
            self.cached_staging = None;

            // Also clear from global cache
            let mut staging_cache = PERSISTENT_STAGING_TREES.lock().await;
            staging_cache.remove(&self.context.merkle_tree);
        }
    }

    async fn process_single_iteration_pipelined_with_cache(
        &mut self,
        num_append_batches: usize,
        num_nullify_batches: usize,
        loop_iteration: usize,
    ) -> Result<usize> {
        let iteration_start = Instant::now();
        let phase1_start = Instant::now();

        // Always fetch queue data from indexer - we need it for batch preparation
        let queue_result = {
            let rpc = self.context.rpc_pool.get_connection().await?;

            if self.cached_staging.is_none() {
                // No cache - wait for indexer to catch up
                forester_utils::utils::wait_for_indexer(&*rpc)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("Indexer failed to catch up before iteration: {}", e)
                    })?;
            } else {
                debug!(
                    "Reusing cached staging (root: {:?}), skipping indexer wait",
                    self.cached_staging.as_ref().map(|(_, root)| &root[..8])
                );
            }

            let mut accounts = rpc
                .get_multiple_accounts(&[self.context.merkle_tree, self.context.output_queue])
                .await?;

            let merkle_tree_account = accounts[0]
                .take()
                .ok_or_else(|| anyhow::anyhow!("Merkle tree account not found"))?;
            let output_queue_account = accounts[1].take();
            drop(rpc);

            self.fetch_queues_with_accounts(
                num_append_batches,
                num_nullify_batches,
                merkle_tree_account,
                output_queue_account,
            )
            .await?
        };

        let pattern = Self::create_interleaving_pattern(num_append_batches, num_nullify_batches);

        let (prep_tx, prep_rx) = tokio::sync::mpsc::channel(50);
        let (proof_tx, proof_rx) = tokio::sync::mpsc::channel(50);

        let proof_config = ProofConfig {
            append_url: self.context.prover_append_url.clone(),
            update_url: self.context.prover_update_url.clone(),
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

        let final_root = self
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

        let metrics = IterationMetrics {
            phase1_duration,
            phase2_duration,
            phase3_duration,
            total_duration,
            append_batches: num_append_batches,
            nullify_batches: num_nullify_batches,
        };

        {
            let mut state = self.shared_state.write().await;
            state.add_iteration_metrics(metrics);
        }

        self.mark_batches_processed(
            queue_result.append_batch_ids,
            queue_result.nullify_batch_ids,
        )
        .await;

        self.validate_root(final_root, "pipelined execution")
            .await?;

        debug!(
            "Iteration complete. Processed {} items with final root: {:?}",
            total_items,
            &final_root[..8]
        );

        Ok(total_items)
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

        let num_nullify_batches =
            Self::count_ready_batches(&tree_data.queue_batches.batches, processed_batches, false);

        let num_append_batches = if let Some(queue_account) = output_queue_account {
            let mut queue_account_data = queue_account.data.clone();
            let queue_data = BatchedQueueAccount::output_from_bytes(&mut queue_account_data)?;
            Self::count_ready_batches(&queue_data.batch_metadata.batches, processed_batches, true)
        } else {
            0
        };

        drop(shared_state);
        Ok((num_append_batches, num_nullify_batches))
    }

    fn count_ready_batches(
        batches: &[light_batched_merkle_tree::batch::Batch; 2],
        processed_batches: &std::collections::HashSet<ProcessedBatchId>,
        is_append: bool,
    ) -> usize {
        let mut total_ready = 0;

        for (batch_idx, batch) in batches.iter().enumerate() {
            let batch_state = batch.get_state();
            if batch_state == light_batched_merkle_tree::batch::BatchState::Inserted {
                continue;
            }

            let num_full_zkp_batches = batch.get_current_zkp_batch_index() as usize;
            let num_inserted_zkps = batch.get_num_inserted_zkps() as usize;

            for zkp_idx in num_inserted_zkps..num_full_zkp_batches {
                let batch_id = ProcessedBatchId {
                    batch_index: batch_idx,
                    zkp_batch_index: zkp_idx as u64,
                    is_append,
                };
                if !processed_batches.contains(&batch_id) {
                    total_ready += 1;
                }
            }
        }

        total_ready
    }

    async fn parse_onchain_accounts(
        &self,
        num_append_batches: usize,
        num_nullify_batches: usize,
        mut merkle_tree_account: Account,
        output_queue_account: Option<Account>,
    ) -> Result<ParsedOnchainState> {
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

        Ok(ParsedOnchainState {
            current_onchain_root,
            append_metadata,
            nullify_metadata,
            append_batch_ids,
            nullify_batch_ids,
        })
    }

    async fn fetch_indexed_queues(
        &self,
        parsed_state: &ParsedOnchainState,
        num_nullify_batches: usize,
    ) -> Result<light_client::indexer::QueueElementsV2Result> {
        let (output_queue_limit, input_queue_limit) = Self::calculate_queue_limits(
            &parsed_state.append_metadata,
            &parsed_state.nullify_metadata,
            num_nullify_batches,
        );

        let mut connection = self.context.rpc_pool.get_connection().await?;
        let indexer = connection.indexer_mut()?;

        let options = light_client::indexer::QueueElementsV2Options {
            output_queue_start_index: None,
            output_queue_limit,
            input_queue_start_index: None,
            input_queue_limit,
            address_queue_start_index: None,
            address_queue_limit: None,
        };

        let queue_elements_response = indexer
            .get_queue_elements_v2(self.context.merkle_tree.to_bytes(), options, None)
            .await?;

        drop(connection);

        Ok(queue_elements_response.value)
    }

    fn construct_result(
        &self,
        parsed_state: ParsedOnchainState,
        output_queue_v2: Option<light_client::indexer::OutputQueueDataV2>,
        input_queue_v2: Option<light_client::indexer::InputQueueDataV2>,
    ) -> Result<QueueFetchResult> {
        // Build staging tree directly from indexer data using sparse tree approach
        let staging = super::tree_state::StagingTree::from_v2_response(
            output_queue_v2.as_ref(),
            input_queue_v2.as_ref(),
        )?;

        tracing::info!(
            "construct_result: Built staging tree from indexer (sparse), root={:?}, on-chain root={:?}",
            &staging.base_root()[..8],
            &parsed_state.current_onchain_root[..8]
        );

        // Verify that indexer's root matches on-chain root
        if staging.base_root() != parsed_state.current_onchain_root {
            tracing::warn!(
                "Indexer root mismatch! indexer={:?}, on-chain={:?}",
                &staging.base_root()[..8],
                &parsed_state.current_onchain_root[..8]
            );
        }

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
        let output_queue_limit = append_metadata.as_ref().map(|(_, queue_data)| {
            (queue_data.leaves_hash_chains.len() * queue_data.zkp_batch_size as usize) as u16
        });

        let input_queue_limit = if num_nullify_batches > 0 {
            nullify_metadata.as_ref().map(|tree_data| {
                (tree_data.zkp_batch_size as usize * tree_data.leaves_hash_chains.len()) as u16
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
                if oq.initial_root != current_onchain_root {
                    let mut photon_root = [0u8; 8];
                    let mut onchain_root = [0u8; 8];
                    photon_root.copy_from_slice(&oq.initial_root[..8]);
                    onchain_root.copy_from_slice(&current_onchain_root[..8]);

                    return Err(CoordinatorError::PhotonStale {
                        queue_type: "output".to_string(),
                        photon_root,
                        onchain_root,
                    }
                    .into());
                }

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
                if iq.initial_root != current_onchain_root {
                    let mut photon_root = [0u8; 8];
                    let mut onchain_root = [0u8; 8];
                    photon_root.copy_from_slice(&iq.initial_root[..8]);
                    onchain_root.copy_from_slice(&current_onchain_root[..8]);

                    return Err(CoordinatorError::PhotonStale {
                        queue_type: "input".to_string(),
                        photon_root,
                        onchain_root,
                    }
                    .into());
                }

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
    ) -> Result<[u8; 32]> {
        let append_leaf_indices: Vec<u64> = if let Some(data) = append_data {
            data.queue_elements
                .iter()
                .map(|elem| elem.leaf_index)
                .collect()
        } else {
            vec![]
        };

        // Smart cache: Reuse cached staging tree when root matches
        // let mut state = if let Some((cached_staging, cached_root)) = self.cached_staging.take() {
        //     let root_matches = cached_root == on_chain_root ;

        //     if root_matches {
        //         // Root matches - our tx hasn't landed yet, cache is valid
        //         // Reuse staging tree with all accumulated updates; batch indices reset for new queue data
        //         debug!(
        //             "Reusing cached staging: root={:?}, {} accumulated updates",
        //             &on_chain_root[..8],
        //             cached_staging.get_updates().len()
        //         );

        //         // Cached tree has all previous updates. Process new queue data starting from batch 0.
        //         PreparationState::with_cached_staging(
        //             append_leaf_indices,
        //             cached_staging,
        //             output_queue_v2,
        //             input_queue_v2,
        //             on_chain_root,
        //         )
        //     } else {
        //         // Root changed - our tx landed, rebuild from fresh
        //         debug!(
        //             "Root changed, rebuilding: cached={:?}, current={:?}",
        //             &cached_root[..8],
        //             &on_chain_root[..8]
        //         );

        //         PreparationState::new(staging, append_leaf_indices)
        //     }
        // } else {
        // No cache - build fresh
        info!(
            "No cached staging tree, building fresh from on-chain root={:?}",
            &on_chain_root[..8]
        );
        let mut state = PreparationState::new(staging, append_leaf_indices);
        // };
        let mut prepared_batches = Vec::with_capacity(pattern.len());

        // DIAGNOSTIC: Log initial staging root before any batch preparation
        debug!(
            "Starting batch preparation: staging.current_root={:?}",
            &state.staging.current_root()[..8]
        );

        for (i, batch_type) in pattern.iter().enumerate() {
            // DIAGNOSTIC: Log staging root before each batch
            debug!(
                "Before preparing batch {} (type={:?}): staging.current_root={:?}",
                i,
                batch_type.as_str(),
                &state.staging.current_root()[..8]
            );

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
            };

            // DIAGNOSTIC: Log staging root after each batch
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

        // TEMPORARY: Disable caching to isolate cache-related issues
        // Cache the staging tree with all its accumulated updates
        // The staging tree contains complete state; new iterations start from batch 0 with new queue data
        let final_root = state.staging.current_root();

        // DISABLED FOR DEBUGGING:
        // let staging_to_cache = (state.staging.clone(), final_root);
        // self.cached_staging = Some(staging_to_cache.clone());
        // Also save to global cache (survives across epochs)
        // {
        //     let mut staging_cache = PERSISTENT_STAGING_TREES.lock().await;
        //     staging_cache.insert(self.context.merkle_tree, staging_to_cache);
        // }

        Ok(final_root)
    }

    async fn generate_proofs_streaming(
        mut prep_rx: tokio::sync::mpsc::Receiver<(usize, PreparedBatch)>,
        proof_tx: tokio::sync::mpsc::Sender<(usize, PreparedBatch, Result<ProofResult>)>,
        config: ProofConfig,
    ) -> Result<Duration> {
        use light_prover_client::proof_types::{
            batch_append::BatchAppendInputsJson, batch_update::update_inputs_string,
        };

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
                        .submit_proof_async(inputs_json, "append")
                        .await
                    {
                        Ok(job_id) => {
                            info!("Batch {} (append) submitted with job_id: {}", idx, job_id);
                            job_count += 1;

                            let client = append_client.clone();
                            let circuit_inputs = circuit_inputs.clone();
                            let handle = tokio::spawn(async move {
                                debug!(
                                    "Polling for append proof completion: batch {}, job {}",
                                    idx, job_id
                                );
                                let result = client.poll_proof_completion(job_id).await;
                                debug!("Append proof polling complete for batch {}", idx);

                                let proof_result = result.and_then(|proof| {
                                    let big_uint = circuit_inputs.new_root.to_biguint()
                                        .ok_or_else(|| light_prover_client::errors::ProverClientError::GenericError(
                                            "Failed to convert new_root to BigUint".to_string()
                                        ))?;
                                    let new_root = light_hasher::bigint::bigint_to_be_bytes_array::<32>(&big_uint)
                                        .map_err(|e| light_prover_client::errors::ProverClientError::GenericError(
                                            format!("Failed to convert new_root to bytes: {}", e)
                                        ))?;

                                    Ok(ProofResult::Append(InstructionDataBatchAppendInputs {
                                        new_root,
                                        compressed_proof: light_compressed_account::instruction_data::compressed_proof::CompressedProof {
                                            a: proof.a,
                                            b: proof.b,
                                            c: proof.c,
                                        },
                                    }))
                                });

                                let _ = proof_tx_clone
                                    .send((
                                        idx,
                                        PreparedBatch::Append(circuit_inputs),
                                        proof_result.map_err(|e| anyhow::anyhow!("{}", e)),
                                    ))
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
                        .submit_proof_async(inputs_json, "update")
                        .await
                    {
                        Ok(job_id) => {
                            info!("Batch {} (nullify) submitted with job_id: {}", idx, job_id);
                            job_count += 1;

                            let client = nullify_client.clone();
                            let circuit_inputs = circuit_inputs.clone();
                            let handle = tokio::spawn(async move {
                                debug!(
                                    "Polling for nullify proof completion: batch {}, job {}",
                                    idx, job_id
                                );
                                let result = client.poll_proof_completion(job_id).await;
                                debug!("Nullify proof polling complete for batch {}", idx);

                                let proof_result = result.and_then(|proof| {
                                    let big_uint = circuit_inputs.new_root.to_biguint()
                                        .ok_or_else(|| light_prover_client::errors::ProverClientError::GenericError(
                                            "Failed to convert new_root to BigUint".to_string()
                                        ))?;
                                    let new_root = light_hasher::bigint::bigint_to_be_bytes_array::<32>(&big_uint)
                                        .map_err(|e| light_prover_client::errors::ProverClientError::GenericError(
                                            format!("Failed to convert new_root to bytes: {}", e)
                                        ))?;

                                    Ok(ProofResult::Nullify(InstructionDataBatchNullifyInputs {
                                        new_root,
                                        compressed_proof: light_compressed_account::instruction_data::compressed_proof::CompressedProof {
                                            a: proof.a,
                                            b: proof.b,
                                            c: proof.c,
                                        },
                                    }))
                                });

                                let _ = proof_tx_clone
                                    .send((
                                        idx,
                                        PreparedBatch::Nullify(circuit_inputs),
                                        proof_result.map_err(|e| anyhow::anyhow!("{}", e)),
                                    ))
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
            }
        }

        info!(
            "All {} proof requests submitted and polling started",
            job_count
        );

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

        const MAX_BATCH_SIZE: usize = 4;

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
                        anyhow::anyhow!(
                            "Proof generation failed for batch {}: {}",
                            next_to_submit,
                            e
                        )
                    }
                })?;

                let batch_type = pattern[next_to_submit];
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

                if ready_pattern.len() >= MAX_BATCH_SIZE {
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

    async fn validate_root(&self, expected_root: [u8; 32], phase: &str) -> Result<()> {
        let current_root = self.get_current_onchain_root().await?;
        if current_root != expected_root {
            let mut expected = [0u8; 8];
            let mut actual = [0u8; 8];
            expected.copy_from_slice(&expected_root[..8]);
            actual.copy_from_slice(&current_root[..8]);

            warn!(
                "Root changed during {} (multi-forester race): expected {:?}, now {:?}",
                phase, expected, actual
            );

            return Err(CoordinatorError::RootChanged {
                phase: phase.to_string(),
                expected,
                actual,
            }
            .into());
        }

        info!("Root validation passed: {:?}", &expected_root[..8]);
        Ok(())
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
        let tree_data = BatchedMerkleTreeAccount::state_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &self.context.merkle_tree.into(),
        )?;

        let on_chain_root = tree_data
            .root_history
            .last()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("No root in tree history"))?;

        let output_queue_batches = if let Some(queue_account) = output_queue_account {
            let mut queue_account_data = queue_account.data.clone();
            let queue_data = BatchedQueueAccount::output_from_bytes(&mut queue_account_data)?;
            queue_data.batch_metadata.batches
        } else {
            [light_batched_merkle_tree::batch::Batch::default(); 2]
        };

        let mut state = self.shared_state.write().await;
        info!("Syncing: on-chain root = {:?}", &on_chain_root[..8]);
        let root_changed = state.current_root != on_chain_root;

        state.reset(
            on_chain_root,
            &tree_data.queue_batches.batches,
            &output_queue_batches,
        );

        Ok(root_changed)
    }

    async fn get_current_onchain_root(&self) -> Result<[u8; 32]> {
        let rpc = self.context.rpc_pool.get_connection().await?;
        let mut account = rpc
            .get_account(self.context.merkle_tree)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Merkle tree account not found"))?;

        let tree_data = BatchedMerkleTreeAccount::state_from_bytes(
            account.data.as_mut_slice(),
            &self.context.merkle_tree.into(),
        )?;

        tree_data
            .root_history
            .last()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("No root in tree history"))
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

                if batch_idx == 0 || zkp_batch_size == 0 {
                    zkp_batch_size = batch.zkp_batch_size as u16;
                    batch_start_index = batch.start_index;
                }

                for i in num_inserted..current_index {
                    let batch_id = ProcessedBatchId {
                        batch_index: batch_idx,
                        zkp_batch_index: i,
                        is_append: false,
                    };

                    if !shared_state.is_batch_processed(&batch_id) {
                        tree_leaves_hash_chains
                            .push(merkle_tree.hash_chain_stores[batch_idx][i as usize]);
                        if collect_nullify_ids {
                            nullify_batch_ids.push(batch_id);
                        }
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
            current_zkp_batch_index: 0,
            batch_start_index,
            leaves_hash_chains: tree_leaves_hash_chains,
        };

        let mut queue_leaves_hash_chains = Vec::new();
        let mut append_batch_ids = Vec::new();

        for (batch_idx, batch) in output_queue.batch_metadata.batches.iter().enumerate() {
            let batch_state = batch.get_state();
            let not_inserted =
                batch_state != light_batched_merkle_tree::batch::BatchState::Inserted;

            if not_inserted {
                let num_inserted = batch.get_num_inserted_zkps();
                let current_index = batch.get_current_zkp_batch_index();

                for i in num_inserted..current_index {
                    let batch_id = ProcessedBatchId {
                        batch_index: batch_idx,
                        zkp_batch_index: i,
                        is_append: true,
                    };

                    if !shared_state.is_batch_processed(&batch_id) {
                        queue_leaves_hash_chains
                            .push(output_queue.hash_chain_stores[batch_idx][i as usize]);
                        if collect_append_ids {
                            append_batch_ids.push(batch_id);
                        }
                    }
                }
            }
        }
        drop(shared_state);

        let queue_data = ParsedQueueData {
            zkp_batch_size: output_queue.batch_metadata.zkp_batch_size as u16,
            pending_batch_index: output_queue.batch_metadata.pending_batch_index as u32,
            num_inserted_zkps: 0,
            current_zkp_batch_index: 0,
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

                if batch_idx == 0 || zkp_batch_size == 0 {
                    zkp_batch_size = batch.zkp_batch_size as u16;
                    batch_start_index = batch.start_index;
                }

                for i in num_inserted..current_index {
                    let batch_id = ProcessedBatchId {
                        batch_index: batch_idx,
                        zkp_batch_index: i,
                        is_append: false,
                    };

                    if !shared_state.is_batch_processed(&batch_id) {
                        leaves_hash_chains
                            .push(merkle_tree.hash_chain_stores[batch_idx][i as usize]);
                        if collect_batch_ids {
                            batch_ids.push(batch_id);
                        }
                    }
                }
            }
        }
        drop(shared_state);

        let current_root = merkle_tree
            .root_history
            .last()
            .ok_or_else(|| anyhow::anyhow!("Merkle tree has no root history"))?;

        Ok((
            ParsedMerkleTreeData {
                next_index: merkle_tree.next_index,
                current_root: *current_root,
                root_history: merkle_tree.root_history.to_vec(),
                zkp_batch_size,
                pending_batch_index: merkle_tree.queue_batches.pending_batch_index as u32,
                num_inserted_zkps: 0,
                current_zkp_batch_index: 0,
                batch_start_index,
                leaves_hash_chains,
            },
            batch_ids,
        ))
    }
}

async fn cleanup_old_epochs(tree: Pubkey, current_epoch: u64) {
    let mut states = PERSISTENT_TREE_STATES.lock().await;

    let mut aggregated_metrics = CumulativeMetrics::default();
    let mut old_epochs_to_remove = Vec::new();

    for ((t, e), shared_state) in states.iter() {
        if *t == tree && *e < current_epoch {
            let state = shared_state.read().await;
            let metrics = state.get_metrics();

            aggregated_metrics.iterations += metrics.iterations;
            aggregated_metrics.total_duration += metrics.total_duration;
            aggregated_metrics.phase1_total += metrics.phase1_total;
            aggregated_metrics.phase2_total += metrics.phase2_total;
            aggregated_metrics.phase3_total += metrics.phase3_total;
            aggregated_metrics.total_append_batches += metrics.total_append_batches;
            aggregated_metrics.total_nullify_batches += metrics.total_nullify_batches;

            if let Some(min) = metrics.min_iteration {
                aggregated_metrics.min_iteration = Some(
                    aggregated_metrics
                        .min_iteration
                        .map(|m| m.min(min))
                        .unwrap_or(min),
                );
            }
            if let Some(max) = metrics.max_iteration {
                aggregated_metrics.max_iteration = Some(
                    aggregated_metrics
                        .max_iteration
                        .map(|m| m.max(max))
                        .unwrap_or(max),
                );
            }

            info!(
                "Aggregating metrics from tree {} epoch {}: {} iterations",
                t, e, metrics.iterations
            );

            old_epochs_to_remove.push((*t, *e));
        }
    }

    if aggregated_metrics.iterations > 0 {
        if let Some(current_state) = states.get(&(tree, current_epoch)) {
            let mut state = current_state.write().await;
            state.merge_metrics(aggregated_metrics);
            info!(
                "Merged metrics from {} old epochs into current epoch",
                old_epochs_to_remove.len()
            );
        }
    }

    for key in old_epochs_to_remove {
        info!("Cleaning up old state for tree {} epoch {}", key.0, key.1);
        states.remove(&key);
    }
}

/// Print cumulative performance summary across all trees.
pub async fn print_cumulative_performance_summary(label: &str) {
    let states = PERSISTENT_TREE_STATES.lock().await;

    let mut total_metrics = CumulativeMetrics::default();
    let mut tree_count = 0;

    for ((tree, epoch), shared_state) in states.iter() {
        let state = shared_state.read().await;
        let metrics = state.get_metrics();
        total_metrics.iterations += metrics.iterations;
        total_metrics.total_duration += metrics.total_duration;
        total_metrics.phase1_total += metrics.phase1_total;
        total_metrics.phase2_total += metrics.phase2_total;
        total_metrics.phase3_total += metrics.phase3_total;
        total_metrics.total_append_batches += metrics.total_append_batches;
        total_metrics.total_nullify_batches += metrics.total_nullify_batches;

        if let Some(min) = metrics.min_iteration {
            total_metrics.min_iteration = Some(
                total_metrics
                    .min_iteration
                    .map(|m| m.min(min))
                    .unwrap_or(min),
            );
        }
        if let Some(max) = metrics.max_iteration {
            total_metrics.max_iteration = Some(
                total_metrics
                    .max_iteration
                    .map(|m| m.max(max))
                    .unwrap_or(max),
            );
        }

        tree_count += 1;

        debug!(
            "Tree {} epoch {}: {} iterations",
            tree, epoch, metrics.iterations
        );
    }

    println!("\n========================================");
    println!("  {}", label.to_uppercase());
    println!("========================================");
    println!("Trees processed:         {}", tree_count);
    total_metrics.print_summary("");
}
