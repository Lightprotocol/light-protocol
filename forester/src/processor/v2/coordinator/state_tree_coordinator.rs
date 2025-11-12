use anyhow::Result;
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_client::{indexer::Indexer, rpc::Rpc};
use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use sysinfo::System;
use tokio::sync::Mutex as TokioMutex;
use tracing::{debug, info, warn};

use forester_utils::{utils::wait_for_indexer, ParsedMerkleTreeData, ParsedQueueData};

use crate::processor::v2::common::BatchContext;

use super::{
    batch_preparation, batch_submission,
    error::CoordinatorError,
    proof_generation::{self, ProofConfig},
    shared_state::{
        create_shared_state, CumulativeMetrics, IterationMetrics, ProcessedBatchId, SharedState,
    },
    tree_state::TreeState,
    types::{AppendQueueData, BatchType, NullifyQueueData, PreparationState, PreparedBatch},
};

static PERSISTENT_TREE_STATES: Lazy<Arc<TokioMutex<HashMap<(Pubkey, u64), SharedState>>>> =
    Lazy::new(|| Arc::new(TokioMutex::new(HashMap::new())));

/// Get current process memory usage in MB.
fn get_process_memory_mb() -> u64 {
    use sysinfo::Pid;
    let mut sys = System::new();
    let pid = Pid::from_u32(std::process::id());
    sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    sys.process(pid)
        .map(|p| p.memory() / 1024 / 1024)
        .unwrap_or(0)
}

/// Main coordinator for state tree batch processing.
pub struct StateTreeCoordinator<R: Rpc> {
    shared_state: SharedState,
    context: BatchContext<R>,
}

impl<R: Rpc> StateTreeCoordinator<R> {
    /// Create new coordinator with persistent state management.
    pub async fn new(context: BatchContext<R>, initial_root: [u8; 32]) -> Self {
        let key = (context.merkle_tree, context.epoch);

        let shared_state = {
            let mut states = PERSISTENT_TREE_STATES.lock().await;
            if let Some(state) = states.get(&key) {
                state.clone()
            } else {
                let new_state = create_shared_state(initial_root);
                states.insert(key, new_state.clone());
                new_state
            }
        };

        cleanup_old_epochs(context.merkle_tree, context.epoch).await;

        Self {
            shared_state,
            context,
        }
    }

    /// Main processing loop - continues until no work remains.
    ///
    /// Returns total number of items processed.
    pub async fn process(&mut self) -> Result<usize> {
        let mut total_items_processed = 0;

        loop {
            self.sync_with_chain().await?;

            let (num_append_batches, num_nullify_batches) = self.check_readiness().await?;

            if num_append_batches == 0 && num_nullify_batches == 0 {
                break;
            }

            match self
                .process_single_iteration(num_append_batches, num_nullify_batches)
                .await
            {
                Ok(items_this_iteration) => {
                    total_items_processed += items_this_iteration;
                }
                Err(e) => {
                    if let Some(coord_err) = e.downcast_ref::<CoordinatorError>() {
                        if coord_err.is_retryable() {
                            if matches!(coord_err, CoordinatorError::PhotonStale { .. }) {
                                debug!("Photon staleness detected, waiting before retry");
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                            }
                            continue;
                        }
                    }
                    return Err(e);
                }
            }
        }

        // Print final summary
        {
            let state = self.shared_state.read().await;
            state.print_performance_summary(&format!(
                "Tree: {}, Epoch: {}",
                self.context.merkle_tree, self.context.epoch
            ));
        }

        Ok(total_items_processed)
    }

    /// Process a single iteration of batch operations.
    async fn process_single_iteration(
        &mut self,
        num_append_batches: usize,
        num_nullify_batches: usize,
    ) -> Result<usize> {
        let iteration_start = Instant::now();

        // Phase 1: Fetch queues and prepare batches
        let phase1_start = Instant::now();

        let rpc = self.context.rpc_pool.get_connection().await?;
        wait_for_indexer(&*rpc)
            .await
            .map_err(|e| anyhow::anyhow!("Indexer failed to catch up: {}", e))?;
        drop(rpc);

        let mem_before = get_process_memory_mb();
        debug!(
            "[MEMORY] Before fetch: {} MB (tree={})",
            mem_before, self.context.merkle_tree
        );

        let (tree_state, append_data, nullify_data, append_batch_ids, nullify_batch_ids) = self
            .fetch_queues(num_append_batches, num_nullify_batches)
            .await?;

        let initial_root = tree_state.current_root();
        let pattern = Self::create_interleaving_pattern(num_append_batches, num_nullify_batches);

        let mem_after_fetch = get_process_memory_mb();
        debug!(
            "[MEMORY] After fetch: {} MB ({} nodes)",
            mem_after_fetch,
            tree_state.node_count()
        );

        let prepared_batches = self.prepare_batches(
            tree_state,
            &pattern,
            append_data.as_ref(),
            nullify_data.as_ref(),
        )?;

        let phase1_duration = phase1_start.elapsed();

        let mem_after_prep = get_process_memory_mb();
        debug!("[MEMORY] After preparation: {} MB", mem_after_prep);

        // Validate root hasn't changed
        self.validate_root(initial_root, "preparation").await?;

        // Phase 2: Generate proofs in parallel
        let phase2_start = Instant::now();
        info!("Phase 2: Parallel proof generation");

        let (append_circuit_inputs, nullify_circuit_inputs) =
            Self::split_prepared_batches(prepared_batches);

        let proof_config = ProofConfig {
            append_url: self.context.prover_append_url.clone(),
            update_url: self.context.prover_update_url.clone(),
            polling_interval: self.context.prover_polling_interval,
            max_wait_time: self.context.prover_max_wait_time,
            api_key: self.context.prover_api_key.clone(),
        };

        let (append_proofs, nullify_proofs) = tokio::join!(
            proof_generation::generate_append_proofs(append_circuit_inputs, &proof_config),
            proof_generation::generate_nullify_proofs(nullify_circuit_inputs, &proof_config)
        );

        let append_proofs = append_proofs?;
        let nullify_proofs = nullify_proofs?;
        let phase2_duration = phase2_start.elapsed();

        info!(
            "Phase 2 complete: {} append proofs, {} nullify proofs in {:?}",
            append_proofs.len(),
            nullify_proofs.len(),
            phase2_duration
        );

        // Validate root again before submission
        self.validate_root(initial_root, "proof generation").await?;

        // Phase 3: Submit transactions
        let phase3_start = Instant::now();
        info!("Phase 3: Transaction submission");

        let total_items = self
            .submit_batches(
                append_proofs,
                nullify_proofs,
                &pattern,
                append_data.as_ref(),
                nullify_data.as_ref(),
            )
            .await?;

        let phase3_duration = phase3_start.elapsed();
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

        self.mark_batches_processed(append_batch_ids, nullify_batch_ids)
            .await;

        Ok(total_items)
    }

    /// Check how many batches are ready for processing.
    async fn check_readiness(&self) -> Result<(usize, usize)> {
        if let (Some(0), Some(0)) = (
            self.context.input_queue_hint,
            self.context.output_queue_hint,
        ) {
            debug!("gRPC hints indicate both queues empty");
            return Ok((0, 0));
        }

        let rpc = self.context.rpc_pool.get_connection().await?;
        let mut merkle_tree_account = rpc
            .get_account(self.context.merkle_tree)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Merkle tree account not found"))?;

        let tree_data = BatchedMerkleTreeAccount::state_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &self.context.merkle_tree.into(),
        )?;

        let shared_state = self.shared_state.read().await;
        let processed_batches = &shared_state.processed_batches;

        let num_nullify_batches =
            Self::count_ready_batches(&tree_data.queue_batches.batches, processed_batches, false);

        let num_append_batches = if let Ok(Some(mut queue_account)) =
            rpc.get_account(self.context.output_queue).await
        {
            let queue_data =
                BatchedQueueAccount::output_from_bytes(queue_account.data.as_mut_slice())?;
            Self::count_ready_batches(&queue_data.batch_metadata.batches, processed_batches, true)
        } else {
            0
        };

        drop(shared_state);
        Ok((num_append_batches, num_nullify_batches))
    }

    /// Count ready batches that haven't been processed yet.
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

    /// Fetch queue data and construct tree state.
    async fn fetch_queues(
        &self,
        num_append_batches: usize,
        num_nullify_batches: usize,
    ) -> Result<(
        TreeState,
        Option<AppendQueueData>,
        Option<NullifyQueueData>,
        Vec<ProcessedBatchId>,
        Vec<ProcessedBatchId>,
    )> {
        let rpc = self.context.rpc_pool.get_connection().await?;

        let mut merkle_tree_account = rpc
            .get_account(self.context.merkle_tree)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Merkle tree account not found"))?;

        let merkle_tree_parsed = BatchedMerkleTreeAccount::state_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &self.context.merkle_tree.into(),
        )?;

        let current_onchain_root = merkle_tree_parsed
            .root_history
            .last()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("No root in tree history"))?;

        // Parse on-chain data
        let (append_metadata, nullify_metadata, append_batch_ids, nullify_batch_ids) =
            if num_append_batches > 0 {
                let mut queue_account = rpc
                    .get_account(self.context.output_queue)
                    .await?
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

        // Fetch indexed queue elements
        let (output_queue_limit, input_queue_limit) =
            Self::calculate_queue_limits(&append_metadata, &nullify_metadata, num_nullify_batches);

        let mut connection = self.context.rpc_pool.get_connection().await?;
        let indexer = connection.indexer_mut()?;

        let queue_elements_response = indexer
            .get_queue_elements_v2(
                self.context.merkle_tree.to_bytes(),
                None,
                output_queue_limit,
                None,
                input_queue_limit,
                None,
                None,
                None,
            )
            .await?;

        drop(connection);

        // Validate and construct queue data
        let (output_queue_v2, input_queue_v2) = Self::validate_queue_responses(
            queue_elements_response.value,
            current_onchain_root,
            &append_metadata,
            &nullify_metadata,
        )?;

        let tree_state =
            TreeState::from_v2_response(output_queue_v2.as_ref(), input_queue_v2.as_ref())?;

        let append_data =
            Self::build_append_data(output_queue_v2.as_ref(), &append_metadata, &self.context)?;
        let nullify_data =
            Self::build_nullify_data(input_queue_v2.as_ref(), &nullify_metadata, &self.context)?;

        Ok((
            tree_state,
            append_data,
            nullify_data,
            append_batch_ids,
            nullify_batch_ids,
        ))
    }

    /// Calculate queue limits for indexer fetch.
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

    /// Validate queue responses from indexer.
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
            let oq = response
                .output_queue
                .ok_or_else(|| anyhow::anyhow!("Expected output queue"))?;

            if oq.initial_root != current_onchain_root {
                return Err(CoordinatorError::PhotonStale {
                    queue_type: "output".to_string(),
                    photon_root: oq.initial_root[..8].try_into().unwrap(),
                    onchain_root: current_onchain_root[..8].try_into().unwrap(),
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
        };

        let input_queue = if nullify_metadata.is_some() {
            let iq = response
                .input_queue
                .ok_or_else(|| anyhow::anyhow!("Expected input queue"))?;

            if iq.initial_root != current_onchain_root {
                return Err(CoordinatorError::PhotonStale {
                    queue_type: "input".to_string(),
                    photon_root: iq.initial_root[..8].try_into().unwrap(),
                    onchain_root: current_onchain_root[..8].try_into().unwrap(),
                }
                .into());
            }

            let tree_data = nullify_metadata.as_ref().unwrap();
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
        };

        Ok((output_queue, input_queue))
    }

    /// Build append queue data from response.
    fn build_append_data(
        output_queue: Option<&light_client::indexer::OutputQueueDataV2>,
        metadata: &Option<(ParsedMerkleTreeData, ParsedQueueData)>,
        context: &BatchContext<R>,
    ) -> Result<Option<AppendQueueData>> {
        if let Some(oq) = output_queue {
            let (_, queue_data) = metadata.as_ref().unwrap();

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

    /// Build nullify queue data from response.
    fn build_nullify_data(
        input_queue: Option<&light_client::indexer::InputQueueDataV2>,
        metadata: &Option<ParsedMerkleTreeData>,
        context: &BatchContext<R>,
    ) -> Result<Option<NullifyQueueData>> {
        if let Some(iq) = input_queue {
            let tree_data = metadata.as_ref().unwrap();

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

    /// Prepare all batches according to the interleaving pattern.
    fn prepare_batches(
        &self,
        tree_state: TreeState,
        pattern: &[BatchType],
        append_data: Option<&AppendQueueData>,
        nullify_data: Option<&NullifyQueueData>,
    ) -> Result<Vec<PreparedBatch>> {
        let append_leaf_indices: Vec<u64> = if let Some(data) = append_data {
            data.queue_elements
                .iter()
                .map(|elem| elem.leaf_index)
                .collect()
        } else {
            vec![]
        };

        let mut state = PreparationState::new(tree_state, append_leaf_indices);
        let mut prepared_batches = Vec::new();

        for (i, batch_type) in pattern.iter().enumerate() {
            match batch_type {
                BatchType::Append => {
                    let append_data =
                        append_data.ok_or_else(|| anyhow::anyhow!("Append data not available"))?;
                    let circuit_inputs =
                        batch_preparation::prepare_append_batch(append_data, &mut state)?;
                    prepared_batches.push(PreparedBatch::Append(circuit_inputs));

                    if i < 3 {
                        debug!(
                            "Prepared append batch {} at position {}: root={:?}",
                            state.append_batch_index - 1,
                            i,
                            &state.current_root[..8]
                        );
                    }
                }
                BatchType::Nullify => {
                    let nullify_data = nullify_data
                        .ok_or_else(|| anyhow::anyhow!("Nullify data not available"))?;
                    let circuit_inputs =
                        batch_preparation::prepare_nullify_batch(nullify_data, &mut state)?;
                    prepared_batches.push(PreparedBatch::Nullify(circuit_inputs));

                    if i < 3 {
                        debug!(
                            "Prepared nullify batch {} at position {}: root={:?}",
                            state.nullify_batch_index - 1,
                            i,
                            &state.current_root[..8]
                        );
                    }
                }
            }
        }

        // Update shared state with computed root
        {
            let mut shared = self.shared_state.blocking_write();
            shared.update_root(state.current_root);
        }

        Ok(prepared_batches)
    }

    /// Split prepared batches into separate append and nullify collections.
    fn split_prepared_batches(
        batches: Vec<PreparedBatch>,
    ) -> (
        Vec<light_prover_client::proof_types::batch_append::BatchAppendsCircuitInputs>,
        Vec<light_prover_client::proof_types::batch_update::BatchUpdateCircuitInputs>,
    ) {
        let mut append_inputs = Vec::new();
        let mut nullify_inputs = Vec::new();

        for batch in batches {
            match batch {
                PreparedBatch::Append(inputs) => append_inputs.push(inputs),
                PreparedBatch::Nullify(inputs) => nullify_inputs.push(inputs),
            }
        }

        (append_inputs, nullify_inputs)
    }

    /// Submit batches to blockchain.
    async fn submit_batches(
        &self,
        append_proofs: Vec<
            light_batched_merkle_tree::merkle_tree::InstructionDataBatchAppendInputs,
        >,
        nullify_proofs: Vec<
            light_batched_merkle_tree::merkle_tree::InstructionDataBatchNullifyInputs,
        >,
        pattern: &[BatchType],
        append_data: Option<&AppendQueueData>,
        nullify_data: Option<&NullifyQueueData>,
    ) -> Result<usize> {
        let total_items = if !append_proofs.is_empty() && !nullify_proofs.is_empty() {
            // Interleaved submission
            batch_submission::submit_interleaved_batches(
                &self.context,
                append_proofs,
                nullify_proofs,
                pattern,
            )
            .await?
        } else {
            let mut total = 0;
            if !append_proofs.is_empty() {
                let zkp_batch_size = append_data.map(|d| d.zkp_batch_size).unwrap_or(10);
                total += batch_submission::submit_append_batches(
                    &self.context,
                    append_proofs,
                    zkp_batch_size,
                )
                .await?;
            }
            if !nullify_proofs.is_empty() {
                let zkp_batch_size = nullify_data.map(|d| d.zkp_batch_size).unwrap_or(10);
                total += batch_submission::submit_nullify_batches(
                    &self.context,
                    nullify_proofs,
                    zkp_batch_size,
                )
                .await?;
            }
            total
        };

        Ok(total_items)
    }

    /// Create interleaving pattern for batch operations.
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

    /// Validate that on-chain root matches expected root.
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

    /// Mark batches as successfully processed.
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

    /// Sync state with on-chain data.
    async fn sync_with_chain(&mut self) -> Result<()> {
        let rpc = self.context.rpc_pool.get_connection().await?;
        let mut account = rpc
            .get_account(self.context.merkle_tree)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Merkle tree account not found"))?;

        let tree_data = BatchedMerkleTreeAccount::state_from_bytes(
            account.data.as_mut_slice(),
            &self.context.merkle_tree.into(),
        )?;

        let on_chain_root = tree_data
            .root_history
            .last()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("No root in tree history"))?;

        let output_queue_batches =
            if let Ok(Some(mut queue_account)) = rpc.get_account(self.context.output_queue).await {
                let queue_data =
                    BatchedQueueAccount::output_from_bytes(queue_account.data.as_mut_slice())?;
                queue_data.batch_metadata.batches
            } else {
                [light_batched_merkle_tree::batch::Batch::default(); 2]
            };

        let mut state = self.shared_state.write().await;
        info!("Syncing: on-chain root = {:?}", &on_chain_root[..8]);

        state.reset(
            on_chain_root,
            &tree_data.queue_batches.batches,
            &output_queue_batches,
        );

        Ok(())
    }

    /// Get current on-chain root.
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

    /// Parse tree and queue data from on-chain accounts (helper method).
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

        let tree_data = ParsedMerkleTreeData {
            next_index: merkle_tree.next_index,
            current_root: *merkle_tree.root_history.last().unwrap(),
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

    /// Parse tree data only (helper method).
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

        Ok((
            ParsedMerkleTreeData {
                next_index: merkle_tree.next_index,
                current_root: *merkle_tree.root_history.last().unwrap(),
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

/// Clean up old epoch states and merge metrics into current epoch.
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
