use std::{
    collections::{HashMap, VecDeque},
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
    merkle_tree::{BatchedMerkleTreeAccount, InstructionDataAddressAppendInputs},
};
use light_client::{
    indexer::{Base58Conversions, Indexer},
    rpc::Rpc,
};
use light_hasher::{bigint::bigint_to_be_bytes_array, Poseidon};
use light_prover_client::proof_client::ProofClient;
use light_sparse_merkle_tree::SparseMerkleTree;
use once_cell::sync::Lazy;
use solana_sdk::{account::Account, pubkey::Pubkey};
use tokio::{sync::Mutex as TokioMutex, time::sleep};
use tracing::{debug, error, info, warn};

use super::{
    batch_utils::{
        self, validate_root, MAX_COORDINATOR_RETRIES, MAX_JOB_NOT_FOUND_RESUBMITS,
        PHOTON_STALE_MAX_RETRIES, PHOTON_STALE_RETRY_DELAY_MS,
    },
    error::CoordinatorError,
    proof_generation::ProofConfig,
    shared_state::{
        create_shared_state, CumulativeMetrics, IterationMetrics, ProcessedBatchId, SharedState,
    },
    types::{AddressQueueData, PreparedBatch},
};
use crate::{errors::ForesterError, metrics, processor::v2::common::BatchContext};
use light_prover_client::proof_types::batch_address_append::BatchAddressAppendInputsJson;

type PersistentAddressTreeStatesCache = Arc<TokioMutex<HashMap<Pubkey, SharedState>>>;
static PERSISTENT_ADDRESS_TREE_STATES: Lazy<PersistentAddressTreeStatesCache> =
    Lazy::new(|| Arc::new(TokioMutex::new(HashMap::new())));

struct OnchainState {
    current_onchain_root: [u8; 32],
    zkp_batch_size: u16,
    start_index: u64,
    batch_ids: VecDeque<ProcessedBatchId>,
    batches: [light_batched_merkle_tree::batch::Batch; 2],
}

pub struct AddressTreeCoordinator<R: Rpc> {
    shared_state: SharedState,
    pub context: BatchContext<R>,
    pending_queue_items: usize,
}

impl<R: Rpc> fmt::Debug for AddressTreeCoordinator<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AddressTreeCoordinator")
            .field("tree", &self.context.merkle_tree)
            .finish()
    }
}

impl<R: Rpc> AddressTreeCoordinator<R> {
    fn update_pending_queue_metric(&self) {
        metrics::update_pending_queue_items(&self.context.merkle_tree, self.pending_queue_items);
    }

    pub fn refresh_epoch_context(&mut self, epoch: u64, epoch_phases: forester_utils::forester_epoch::EpochPhases) {
        self.context.epoch = epoch;
        self.context.epoch_phases = epoch_phases;
    }

    pub fn on_queue_update(&mut self, queue_size: u64) {
        self.pending_queue_items = queue_size.min(usize::MAX as u64) as usize;
        self.update_pending_queue_metric();
    }

    fn consume_processed_items(&mut self, processed: usize) {
        if processed > 0 {
            let before = self.pending_queue_items;
            self.pending_queue_items = self.pending_queue_items.saturating_sub(processed);
            debug!(
                "Pending address items adjusted: {} -> {} (processed {})",
                before, self.pending_queue_items, processed
            );
        }
        self.update_pending_queue_metric();
        if self.pending_queue_items == 0 {
            debug!(
                "Address queue drained for tree {}",
                self.context.merkle_tree
            );
        }
    }

    pub async fn new(context: BatchContext<R>, initial_root: [u8; 32]) -> Self {
        let key = context.merkle_tree;

        let shared_state = {
            let mut states = PERSISTENT_ADDRESS_TREE_STATES.lock().await;
            if let Some(state) = states.get(&key) {
                state.clone()
            } else {
                let new_state = create_shared_state(initial_root);
                states.insert(key, new_state.clone());
                new_state
            }
        };

        info!(
            "AddressTreeCoordinator instance created: tree={}, epoch={}",
            context.merkle_tree,
            context.epoch,
        );

        Self {
            shared_state,
            context,
            pending_queue_items: 0,
        }
    }

    pub async fn process(&mut self) -> Result<usize> {
        let mut total_items_processed = 0;
        let mut consecutive_retries = 0;

        debug!(
            "AddressTreeCoordinator::process() called for tree={}, epoch={}",
            self.context.merkle_tree, self.context.epoch
        );

        loop {
            let iteration_inputs = self.prepare_iteration_inputs().await?;
            let (num_batches, address_data) = if let Some(inputs) = iteration_inputs {
                inputs
            } else {
                break;
            };

            if num_batches == 0 {
                break;
            }

            match self
                .process_single_iteration(address_data)
                .await
            {
                Ok(items_this_iteration) => {
                    total_items_processed += items_this_iteration;
                    consecutive_retries = 0;

                    // If we had batches to process but processed 0 items, it means
                    // the active phase ended, and we couldn't submit. Exit the loop.
                    if num_batches > 0 && items_this_iteration == 0 {
                        info!(
                            "Active phase ended with {} batches remaining unprocessed. Exiting epoch {} processing.",
                            num_batches, self.context.epoch
                        );
                        break;
                    }
                }
                Err(e) => {
                    if let Some(coord_err) = e.downcast_ref::<CoordinatorError>() {
                        if coord_err.is_retryable() {
                            consecutive_retries += 1;

                            if consecutive_retries >= MAX_COORDINATOR_RETRIES {
                                warn!(
                                    "Max consecutive retries ({}) reached for error: {}. Giving up on this batch.",
                                    MAX_COORDINATOR_RETRIES, coord_err
                                );
                                break;
                            }

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

        {
            let state = self.shared_state.read().await;
            state.print_performance_summary(&format!(
                "Address Tree: {}, Epoch: {}",
                self.context.merkle_tree, self.context.epoch
            ));
        }

        Ok(total_items_processed)
    }

    async fn prepare_iteration_inputs(&mut self) -> Result<Option<(usize, AddressQueueData)>> {
        let rpc = self.context.rpc_pool.get_connection().await?;
        let mut merkle_tree_account = rpc
            .get_account(self.context.merkle_tree)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Address merkle tree account not found"))?;
        drop(rpc);

        self.sync_with_chain(&mut merkle_tree_account).await?;

        let num_batches = self.check_readiness(&mut merkle_tree_account).await?;

        if num_batches == 0 {
            return Ok(None);
        }

        let address_data = self
            .fetch_address_queue()
            .await?;

        Ok(Some((num_batches, address_data)))
    }

    async fn process_single_iteration(
        &mut self,
        address_data: AddressQueueData,
    ) -> Result<usize> {
        let iteration_start = Instant::now();
        let total_batches = address_data.leaves_hash_chains.len();

        let (prep_tx, prep_rx) = tokio::sync::mpsc::channel(50);
        let (proof_tx, proof_rx) = tokio::sync::mpsc::channel(50);

        let proof_config = ProofConfig {
            append_url: String::new(),
            update_url: String::new(), // Not used for address trees
            address_append_url: self.context.prover_address_append_url.clone(),
            polling_interval: self.context.prover_polling_interval,
            max_wait_time: self.context.prover_max_wait_time,
            api_key: self.context.prover_api_key.clone(),
        };

        let proof_gen_handle = tokio::spawn(async move {
            Self::generate_proofs_streaming(prep_rx, proof_tx, proof_config).await
        });

        let prepare_future = self.prepare_batches_streaming(&address_data, prep_tx);
        let submit_future = self.submit_proofs_streaming(proof_rx, address_data.zkp_batch_size);

        let ((final_root, phase1_duration), (total_items, phase3_duration)) =
            tokio::try_join!(prepare_future, submit_future)?;

        let submitted_batches = total_items
            .checked_div(address_data.zkp_batch_size as usize)
            .unwrap_or(0);

        info!(
            "Submitted {} out of {} address batches for tree {} (total_items={}, zkp_batch_size={})",
            submitted_batches, total_batches, self.context.merkle_tree, total_items, address_data.zkp_batch_size
        );

        {
            let mut state = self.shared_state.write().await;
            if submitted_batches == total_batches {
                state.update_root(final_root);
            } else if submitted_batches > 0 {
                debug!(
                    "Submitted {}/{} address batches for tree {}",
                    submitted_batches, total_batches, self.context.merkle_tree
                );
            }

            info!(
                "Marking {}/{} batches as processed",
                submitted_batches,
                address_data.batch_ids.len()
            );
            for (i, batch_id) in address_data.batch_ids.iter().take(submitted_batches).enumerate() {
                debug!(
                    "  marking batch_id[{}] as processed: batch_index={}, zkp_batch_index={}, start_leaf_index={:?}",
                    i, batch_id.batch_index, batch_id.zkp_batch_index, batch_id.start_leaf_index
                );
                state.mark_batch_processed(*batch_id);
            }
        }

        let phase2_duration = proof_gen_handle.await??;
        let total_duration = iteration_start.elapsed();

        let metrics = IterationMetrics {
            phase1_duration,
            phase2_duration,
            phase3_duration,
            total_duration,
            append_batches: submitted_batches,
            nullify_batches: 0,
        };
        metrics::observe_iteration_duration(&self.context.merkle_tree, total_duration);

        {
            let mut state = self.shared_state.write().await;
            state.add_iteration_metrics(metrics);
        }

        if submitted_batches > 0 {
            let current_root = self.get_current_onchain_root().await?;
            validate_root(current_root, final_root, "address tree execution")?;
            self.consume_processed_items(total_items);
            metrics::increment_batches_processed(
                &self.context.merkle_tree,
                "address",
                submitted_batches,
            );
        }

        debug!(
            "Iteration complete. Submitted {} address batches ({} items)",
            submitted_batches, total_items
        );

        Ok(total_items)
    }

    async fn check_readiness(&self, merkle_tree_account: &mut Account) -> Result<usize> {
        let tree_data = BatchedMerkleTreeAccount::address_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &self.context.merkle_tree.into(),
        )?;

        let shared_state = self.shared_state.read().await;
        let processed_batches = &shared_state.processed_batches;

        let num_batches = batch_utils::count_ready_batches(
            &tree_data.queue_batches.batches,
            processed_batches,
            false, // is_append (address trees are not append)
            true,  // calculate_start_index (needed for address trees)
        )
            .min(1);

        drop(shared_state);
        Ok(num_batches)
    }


    async fn parse_onchain_account(
        &self,
        mut merkle_tree_account: Account,
    ) -> Result<OnchainState> {
        let merkle_tree_parsed = BatchedMerkleTreeAccount::address_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &self.context.merkle_tree.into(),
        )?;

        let current_onchain_root = merkle_tree_parsed
            .root_history
            .last()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("No root in tree history"))?;

        let shared_state = self.shared_state.write().await;

        let mut batch_ids = Vec::new();
        let mut zkp_batch_size = 0u16;
        let mut start_index = 0u64;
        let mut start_index_set = false;

        let tree_next_index = merkle_tree_parsed.next_index;
        let num_addresses_in_tree = if tree_next_index > 1 {
            tree_next_index - 1
        } else {
            0
        };

        for (batch_idx, batch) in merkle_tree_parsed.queue_batches.batches.iter().enumerate() {
            let batch_state = batch.get_state();
            let not_inserted =
                batch_state != light_batched_merkle_tree::batch::BatchState::Inserted;

            if not_inserted {
                let num_inserted = batch.get_num_inserted_zkps();
                let current_index = batch.get_current_zkp_batch_index();

                if batch_idx == 0 || zkp_batch_size == 0 {
                    zkp_batch_size = batch.zkp_batch_size as u16;
                }

                tracing::debug!(
                    "Address batch {} state={:?} num_inserted={} current_index={} start_index={} (on-chain stale), tree_next_index={}, num_addresses_in_tree={}",
                    batch_idx,
                    batch_state,
                    num_inserted,
                    current_index,
                    batch.start_index,
                    tree_next_index,
                    num_addresses_in_tree
                );

                if current_index > num_inserted {
                    let first_uninserted_start = tree_next_index;

                    if !start_index_set {
                        start_index = first_uninserted_start;
                        start_index_set = true;
                    }

                    for i in num_inserted..current_index {
                        let start_leaf_index = first_uninserted_start + ((i - num_inserted) * batch.zkp_batch_size);

                        tracing::debug!(
                            "batch_id: batch_index={}, zkp_batch_index={}, start_leaf_index={} (calculated: tree.next_index={} + ({} - {}) * {})",
                            batch_idx,
                            i,
                            start_leaf_index,
                            tree_next_index,
                            i,
                            num_inserted,
                            batch.zkp_batch_size
                        );

                        let batch_id = ProcessedBatchId {
                            batch_index: batch_idx,
                            zkp_batch_index: i,
                            is_append: false,
                            start_leaf_index: Some(start_leaf_index),
                        };

                        if !shared_state.is_batch_processed(&batch_id) {
                            batch_ids.push(batch_id);
                        }
                    }
                }
            }
        }
        drop(shared_state);

        for (i, batch_id) in batch_ids.iter().enumerate() {
            debug!(
                "created batch_id[{}]: batch_index={}, zkp_batch_index={}, start_leaf_index={:?}",
                i, batch_id.batch_index, batch_id.zkp_batch_index, batch_id.start_leaf_index
            );
        }

        Ok(OnchainState {
            current_onchain_root,
            zkp_batch_size,
            start_index: if start_index_set {
                start_index
            } else {
                merkle_tree_parsed.next_index
            },
            batch_ids: VecDeque::from(batch_ids),
            batches: merkle_tree_parsed.queue_batches.batches.clone(),
        })
    }

    async fn fetch_address_queue(
        &mut self,
    ) -> Result<AddressQueueData> {
        for attempt in 0..=PHOTON_STALE_MAX_RETRIES {
            return match self
                .fetch_address_queue_inner()
                .await
            {
                Ok(data) => Ok(data),
                Err(err) => {
                    if let Some(coord_err) = err.downcast_ref::<CoordinatorError>() {
                        if let CoordinatorError::PhotonStale {
                            queue_type,
                            photon_root,
                            onchain_root,
                        } = coord_err
                        {
                            if attempt < PHOTON_STALE_MAX_RETRIES {
                                warn!(
                                    "Photon staleness detected for {} queue (tree={}, attempt {}/{}): photon_root={:?}, on_chain_root={:?}. Retrying in {}ms",
                                    queue_type,
                                    self.context.merkle_tree,
                                    attempt + 1,
                                    PHOTON_STALE_MAX_RETRIES,
                                    photon_root,
                                    onchain_root,
                                    PHOTON_STALE_RETRY_DELAY_MS
                                );
                                sleep(Duration::from_millis(PHOTON_STALE_RETRY_DELAY_MS)).await;
                                continue;
                            }
                        }
                    }
                    Err(err)
                }
            }
        }

        Err(anyhow::anyhow!(
            "Exceeded Photon staleness retries for tree {}",
            self.context.merkle_tree
        ))
    }

    async fn fetch_address_queue_inner(
        &mut self,
    ) -> Result<AddressQueueData> {
        let rpc = self.context.rpc_pool.get_connection().await?;
        let account = rpc
            .get_account(self.context.merkle_tree)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Address merkle tree account not found"))?;
        drop(rpc);

        let mut parsed_state = self
            .parse_onchain_account(account)
            .await?;

        let num_batches = {
            let shared_state = self.shared_state.read().await;
            let processed_batches = &shared_state.processed_batches;

            let num_batches = batch_utils::count_ready_batches(
                &parsed_state.batches,
                processed_batches,
                false, // is_append (address trees are not append)
                true,  // calculate_start_index (needed for address trees)
            )
                .min(1);

            drop(shared_state);
           num_batches
        };

        let total_elements = num_batches * parsed_state.zkp_batch_size as usize;

        debug!(
            "Requesting {} address elements from Photon (num_batches={}, zkp_batch_size={})",
            total_elements, num_batches, parsed_state.zkp_batch_size
        );

        let mut connection = self.context.rpc_pool.get_connection().await?;
        let indexer = connection.indexer_mut()?;

        // Calculate the queue start index: for address trees, queue_index = leaf_index - 1
        // (first address in queue goes to leaf position 1)
        // Use the first batch_id's start_leaf_index to get the correct starting position
        let address_queue_start_idx = if let Some(first_batch_id) = parsed_state.batch_ids.front() {
            if let Some(start_leaf) = first_batch_id.start_leaf_index {
                if start_leaf > 1 {
                    Some(start_leaf - 1)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let options = light_client::indexer::QueueElementsV2Options {
            output_queue_start_index: None,
            output_queue_limit: None,
            input_queue_start_index: None,
            input_queue_limit: None,
            address_queue_start_index: address_queue_start_idx,
            address_queue_limit: Some(total_elements as u16),
            address_queue_zkp_batch_size: Some(parsed_state.zkp_batch_size),
        };

        let queue_elements_response = indexer
            .get_queue_elements_v2(self.context.merkle_tree.to_bytes(), options, None)
            .await?;

        drop(connection);

        let address_queue_v2 = queue_elements_response
            .value
            .address_queue
            .ok_or_else(|| anyhow::anyhow!("No address queue in indexer response"))?;

        debug!(
            "Fetched address queue: start_index={}, addresses={}, first_queue_idx={:?}, last_queue_idx={:?}",
            address_queue_v2.start_index,
            address_queue_v2.addresses.len(),
            address_queue_v2.queue_indices.first(),
            address_queue_v2.queue_indices.last()
        );

        if address_queue_v2.initial_root != parsed_state.current_onchain_root {
            let mut photon_root = [0u8; 8];
            let mut onchain_root = [0u8; 8];
            photon_root.copy_from_slice(&address_queue_v2.initial_root[..8]);
            onchain_root.copy_from_slice(&parsed_state.current_onchain_root[..8]);

            return Err(CoordinatorError::PhotonStale {
                queue_type: "address".to_string(),
                photon_root,
                onchain_root,
            }
            .into());
        }

        if address_queue_v2.low_element_proofs.len() != address_queue_v2.low_element_indices.len() {
            return Err(anyhow::anyhow!(
                "Address queue missing proofs: expected {} entries, got {}",
                address_queue_v2.low_element_indices.len(),
                address_queue_v2.low_element_proofs.len()
            ));
        }
        let low_element_proofs = address_queue_v2.low_element_proofs.clone();
        let subtrees = address_queue_v2.subtrees.clone();

        let all_leaves_hash_chains: Vec<[u8; 32]> = address_queue_v2
            .leaves_hash_chains
            .iter()
            .map(|h| h.to_bytes())
            .collect();

        let offset_addresses = parsed_state.start_index.saturating_sub(address_queue_v2.start_index);
        let offset_batches = (offset_addresses / parsed_state.zkp_batch_size as u64) as usize;

        let leaves_hash_chains: Vec<[u8; 32]> = all_leaves_hash_chains
            .into_iter()
            .skip(offset_batches)
            .collect();
        let batch_count = leaves_hash_chains.len();

        let mut batch_ids = Vec::with_capacity(batch_count);
        for _ in 0..batch_count {
            if let Some(id) = parsed_state.batch_ids.pop_front() {
                batch_ids.push(id);
            } else {
                break;
            }
        }

        let skip_addresses = offset_batches * parsed_state.zkp_batch_size as usize;
        let addresses: Vec<[u8; 32]> = address_queue_v2.addresses.into_iter().skip(skip_addresses).collect();
        let low_element_values: Vec<[u8; 32]> = address_queue_v2.low_element_values.into_iter().skip(skip_addresses).collect();
        let low_element_next_values: Vec<[u8; 32]> = address_queue_v2.low_element_next_values.into_iter().skip(skip_addresses).collect();
        let low_element_indices: Vec<u64> = address_queue_v2.low_element_indices.into_iter().skip(skip_addresses).collect();
        let low_element_next_indices: Vec<u64> = address_queue_v2.low_element_next_indices.into_iter().skip(skip_addresses).collect();
        let low_element_proofs_filtered: Vec<Vec<[u8; 32]>> = low_element_proofs.into_iter().skip(skip_addresses).collect();

        Ok(AddressQueueData {
            addresses,
            low_element_values,
            low_element_next_values,
            low_element_indices,
            low_element_next_indices,
            low_element_proofs: low_element_proofs_filtered,
            leaves_hash_chains,
            zkp_batch_size: parsed_state.zkp_batch_size,
            subtrees,
            start_index: parsed_state.start_index,
            batch_ids,
        })
    }

    async fn prepare_batches_streaming(
        &self,
        address_data: &AddressQueueData,
        tx: tokio::sync::mpsc::Sender<(usize, PreparedBatch)>,
    ) -> Result<([u8; 32], Duration)> {
        let prepare_start = Instant::now();

        let subtrees_array: [[u8; 32]; DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize] = address_data
            .subtrees
            .clone()
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to convert subtrees to array"))?;

        let mut sparse_merkle_tree =
            SparseMerkleTree::<Poseidon, { DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize }>::new(
                subtrees_array,
                address_data.start_index as usize,
            );

        let mut current_root = {
            let state = self.shared_state.read().await;
            state.current_root
        };

        let mut next_index = address_data.start_index as usize;

        let mut changelog: Vec<
            light_sparse_merkle_tree::changelog::ChangelogEntry<
                { DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize },
            >,
        > = Vec::new();
        let mut indexed_changelog: Vec<
            light_sparse_merkle_tree::indexed_changelog::IndexedChangelogEntry<
                usize,
                { DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize },
            >,
        > = Vec::new();
        let batch_size = address_data.zkp_batch_size as usize;

        for (batch_idx, leaves_hash_chain) in address_data.leaves_hash_chains.iter().enumerate() {
            let start_idx = batch_idx * batch_size;
            let end_idx = start_idx + batch_size;

            let batch_addresses = address_data.addresses[start_idx..end_idx].to_vec();
            let low_element_values = address_data.low_element_values[start_idx..end_idx].to_vec();
            let low_element_next_values =
                address_data.low_element_next_values[start_idx..end_idx].to_vec();
            let low_element_indices = address_data.low_element_indices[start_idx..end_idx]
                .iter()
                .map(|idx| *idx as usize)
                .collect::<Vec<_>>();
            let low_element_next_indices = address_data.low_element_next_indices
                [start_idx..end_idx]
                .iter()
                .map(|idx| *idx as usize)
                .collect::<Vec<_>>();
            let low_element_proofs = address_data.low_element_proofs[start_idx..end_idx].to_vec();

            let adjusted_start_index = next_index;

            let batch_len = batch_addresses.len();

            tracing::debug!(
                "Preparing address batch {}: adjusted_start_index={}, current_root={:?}, low_idx_sample={:?}",
                batch_idx,
                adjusted_start_index,
                &current_root[..8],
                &low_element_indices[..low_element_indices.len().min(3)]
            );

            let inputs = light_prover_client::proof_types::batch_address_append::get_batch_address_append_circuit_inputs(
                adjusted_start_index,
                current_root,
                low_element_values.clone(),
                low_element_next_values.clone(),
                low_element_indices.clone(),
                low_element_next_indices.clone(),
                low_element_proofs.clone(),
                batch_addresses.clone(),
                &mut sparse_merkle_tree,
                *leaves_hash_chain,
                batch_size,
                &mut changelog,
                &mut indexed_changelog,
            )
            .map_err(|e| anyhow::anyhow!("Failed to get circuit inputs: {}", e))?;

            let new_root_bytes = bigint_to_be_bytes_array::<32>(&inputs.new_root)
                .map_err(|e| anyhow::anyhow!("Failed to convert new_root to bytes: {}", e))?;

            current_root = new_root_bytes;
            next_index = next_index
                .checked_add(batch_len)
                .ok_or_else(|| anyhow::anyhow!("Address batch index overflow"))?;

            tx.send((batch_idx, PreparedBatch::Address(inputs)))
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send prepared batch: {}", e))?;
        }

        let sparse_tree_final_root = sparse_merkle_tree.root();
        if sparse_tree_final_root != current_root {
            debug!(
                "Sparse tree root {:?} differs from patched circuit root {:?} (expected when changelog rewrites earlier leaves)",
                &sparse_tree_final_root[..8],
                &current_root[..8]
            );
        }

        Ok((current_root, prepare_start.elapsed()))
    }

    async fn generate_proofs_streaming(
        mut prep_rx: tokio::sync::mpsc::Receiver<(usize, PreparedBatch)>,
        proof_tx: tokio::sync::mpsc::Sender<(usize, Result<InstructionDataAddressAppendInputs>)>,
        config: ProofConfig,
    ) -> Result<Duration> {

        let client = Arc::new(ProofClient::with_config(
            config.address_append_url.clone(),
            config.polling_interval,
            config.max_wait_time,
            config.api_key.clone(),
        ));

        let mut poll_handles = Vec::new();
        let proof_gen_start = Instant::now();
        let mut job_count = 0;

        while let Some((idx, prepared_batch)) = prep_rx.recv().await {
            let proof_tx_clone = proof_tx.clone();

            if let PreparedBatch::Address(circuit_inputs) = prepared_batch {
                debug!("Submitting address proof request for batch {}", idx);
                let inputs_json =
                    BatchAddressAppendInputsJson::from_inputs(&circuit_inputs).to_string();
                match client
                    .submit_proof_async(inputs_json.clone(), "address")
                    .await
                {
                    Ok(job_id) => {
                        info!("Batch {} (address) submitted with job_id: {}", idx, job_id);
                        job_count += 1;

                        let client_clone = client.clone();
                        let inputs_json_clone = inputs_json.clone();
                        let handle = tokio::spawn(async move {
                            debug!(
                                "Polling for address proof completion: batch {}, job {}",
                                idx, job_id
                            );
                            let mut current_job = job_id;
                            let mut resubmits = 0usize;

                            loop {
                                let result = client_clone
                                    .poll_proof_completion(current_job.clone())
                                    .await;
                                debug!(
                                    "Address proof polling complete for batch {} (job {})",
                                    idx, current_job
                                );

                                match result {
                                    Ok(proof) => {
                                        let proof_result = (|| {
                                            let new_root = bigint_to_be_bytes_array::<32>(
                                                &circuit_inputs.new_root,
                                            )
                                            .map_err(|e| light_prover_client::errors::ProverClientError::GenericError(
                                                format!("Failed to convert new_root to bytes: {}", e)
                                            ))?;

                                            Ok(InstructionDataAddressAppendInputs {
                                                new_root,
                                                compressed_proof: light_compressed_account::instruction_data::compressed_proof::CompressedProof {
                                                    a: proof.a,
                                                    b: proof.b,
                                                    c: proof.c,
                                                },
                                            })
                                        })(
                                        );

                                        let _ = proof_tx_clone
                                            .send((
                                                idx,
                                                proof_result.map_err(|e: light_prover_client::errors::ProverClientError| anyhow::anyhow!("{}", e)),
                                            ))
                                            .await;
                                        break;
                                    }
                                    Err(e)
                                        if e.to_string().contains("job_not_found")
                                            && resubmits < MAX_JOB_NOT_FOUND_RESUBMITS =>
                                    {
                                        resubmits += 1;
                                        warn!(
                                            "Address proof job {} not found (batch {}), resubmitting attempt {}/{}",
                                            current_job, idx, resubmits, MAX_JOB_NOT_FOUND_RESUBMITS
                                        );
                                        match client_clone
                                            .submit_proof_async(
                                                inputs_json_clone.clone(),
                                                "address",
                                            )
                                            .await
                                        {
                                            Ok(new_job_id) => {
                                                info!(
                                                    "Batch {} resubmitted with new job_id {}",
                                                    idx, new_job_id
                                                );
                                                current_job = new_job_id;
                                                continue;
                                            }
                                            Err(submit_err) => {
                                                let _ = proof_tx_clone
                                                    .send((
                                                        idx,
                                                        Err(anyhow::anyhow!("{}", submit_err)),
                                                    ))
                                                    .await;
                                                break;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = proof_tx_clone
                                            .send((idx, Err(anyhow::anyhow!("{}", e))))
                                            .await;
                                        break;
                                    }
                                }
                            }
                        });
                        poll_handles.push(handle);
                    }
                    Err(e) => {
                        error!("Failed to submit batch {} (address): {}", idx, e);
                        let _ = proof_tx_clone
                            .send((idx, Err(anyhow::anyhow!("{}", e))))
                            .await;
                    }
                }
            }
        }

        info!(
            "All {} address proof requests submitted and polling started",
            job_count
        );

        for handle in poll_handles {
            handle
                .await
                .map_err(|e| anyhow::anyhow!("Proof polling task join error: {}", e))?;
        }

        let proof_gen_duration = proof_gen_start.elapsed();

        info!(
            "Phase 2 complete: All address proofs received in {:?}",
            proof_gen_duration
        );

        Ok(proof_gen_duration)
    }

    async fn submit_proofs_streaming(
        &self,
        mut proof_rx: tokio::sync::mpsc::Receiver<(
            usize,
            Result<InstructionDataAddressAppendInputs>,
        )>,
        zkp_batch_size: u16,
    ) -> Result<(usize, Duration)> {
        use std::collections::BTreeMap;

        const BATCH_SUBMIT_TIMEOUT_SECS: u64 = 2;

        let mut buffer: BTreeMap<usize, Result<InstructionDataAddressAppendInputs>> =
            BTreeMap::new();
        let mut next_to_submit = 0;
        let mut total_items = 0;
        let mut total_submit_duration = Duration::ZERO;
        let mut ready_proofs = Vec::new();
        let mut last_submit_time = Instant::now();

        loop {
            let timeout_duration = Duration::from_secs(BATCH_SUBMIT_TIMEOUT_SECS);
            let time_since_last_submit = last_submit_time.elapsed();
            let remaining_timeout = timeout_duration.saturating_sub(time_since_last_submit);

            let recv_result = if remaining_timeout.is_zero() {
                None
            } else {
                // Wait for next proof or timeout
                tokio::time::timeout(remaining_timeout, proof_rx.recv())
                    .await
                    .ok()
                    .flatten()
            };

            let should_force_submit = recv_result.is_none() && !ready_proofs.is_empty();

            if let Some((idx, proof_result)) = recv_result {
                buffer.insert(idx, proof_result);

                while let Some(proof_result) = buffer.remove(&next_to_submit) {
                    let proof = proof_result.map_err(|e| {
                        anyhow::anyhow!(
                            "Proof generation failed for batch {}: {}",
                            next_to_submit,
                            e
                        )
                    })?;

                    ready_proofs.push(proof);
                    next_to_submit += 1;

                    if !ready_proofs.is_empty() {
                        let submit_start = Instant::now();
                        match super::batch_submission::submit_address_batches(
                            &self.context,
                            std::mem::take(&mut ready_proofs),
                            zkp_batch_size,
                        )
                        .await
                        {
                            Ok(processed) => {
                                total_items += processed;
                                total_submit_duration += submit_start.elapsed();
                                last_submit_time = Instant::now();
                            }
                            Err(e) if Self::is_inactive_phase_error(&e) => {
                                info!(
                                    "Active phase ended before submitting address batches; deferring remaining proofs"
                                );
                                return Ok((total_items, total_submit_duration));
                            }
                            Err(e) => return Err(CoordinatorError::TransactionFailed(e).into()),
                        }
                    }
                }
            } else if should_force_submit {
                // Timeout reached, submit whatever we have
                debug!(
                    "Batch submit timeout reached ({} secs), submitting {} ready address batches",
                    BATCH_SUBMIT_TIMEOUT_SECS,
                    ready_proofs.len()
                );
                let submit_start = Instant::now();
                match super::batch_submission::submit_address_batches(
                    &self.context,
                    std::mem::take(&mut ready_proofs),
                    zkp_batch_size,
                )
                .await
                {
                    Ok(processed) => {
                        total_items += processed;
                        total_submit_duration += submit_start.elapsed();
                        last_submit_time = Instant::now();
                    }
                    Err(e) if Self::is_inactive_phase_error(&e) => {
                        info!(
                            "Active phase ended before submitting address batches; deferring remaining proofs"
                        );
                        return Ok((total_items, total_submit_duration));
                    }
                    Err(e) => return Err(CoordinatorError::TransactionFailed(e).into()),
                }
            } else if recv_result.is_none() {
                // Channel closed and no more proofs to submit
                break;
            }
        }

        if !ready_proofs.is_empty() {
            let submit_start = Instant::now();
            match super::batch_submission::submit_address_batches(
                &self.context,
                ready_proofs,
                zkp_batch_size,
            )
            .await
            {
                Ok(processed) => {
                    total_items += processed;
                    total_submit_duration += submit_start.elapsed();
                }
                Err(e) if Self::is_inactive_phase_error(&e) => {
                    info!(
                        "Active phase ended before submitting remaining address batches; deferring proofs"
                    );
                    return Ok((total_items, total_submit_duration));
                }
                Err(e) => return Err(CoordinatorError::TransactionFailed(e).into()),
            }
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

    fn is_inactive_phase_error(err: &anyhow::Error) -> bool {
        matches!(
            err.downcast_ref::<ForesterError>(),
            Some(ForesterError::NotInActivePhase)
        )
    }

    async fn sync_with_chain(&mut self, merkle_tree_account: &mut Account) -> Result<bool> {
        let tree_data = BatchedMerkleTreeAccount::address_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &self.context.merkle_tree.into(),
        )?;

        let on_chain_root = tree_data
            .root_history
            .last()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("No root in tree history"))?;

        let mut state = self.shared_state.write().await;
        info!("Syncing: on-chain root = {:?}", &on_chain_root[..8]);
        let root_changed = state.current_root != on_chain_root;

        state.reset(
            on_chain_root,
            &tree_data.queue_batches.batches,
            &[light_batched_merkle_tree::batch::Batch::default(); 2],
        );

        Ok(root_changed)
    }

    async fn get_current_onchain_root(&self) -> Result<[u8; 32]> {
        let rpc = self.context.rpc_pool.get_connection().await?;
        let mut account = rpc
            .get_account(self.context.merkle_tree)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Address merkle tree account not found"))?;

        let tree_data = BatchedMerkleTreeAccount::address_from_bytes(
            account.data.as_mut_slice(),
            &self.context.merkle_tree.into(),
        )?;

        tree_data
            .root_history
            .last()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("No root in tree history"))
    }
}

pub async fn print_cumulative_performance_summary(label: &str) {
    let states = PERSISTENT_ADDRESS_TREE_STATES.lock().await;

    let mut total_metrics = CumulativeMetrics::default();
    let mut tree_count = 0;

    for (tree, shared_state) in states.iter() {
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

        debug!("Address Tree {}: {} iterations", tree, metrics.iterations);
    }

    println!("\n========================================");
    println!("  {}", label.to_uppercase());
    println!("========================================");
    println!("Address trees processed: {}", tree_count);
    println!("Total iterations:        {}", total_metrics.iterations);
    println!(
        "Total duration:          {:?}",
        total_metrics.total_duration
    );
    println!(
        "Avg iteration:           {:?}",
        total_metrics.avg_iteration_duration()
    );

    if let Some(min) = total_metrics.min_iteration {
        println!("Min iteration:           {:?}", min);
    }
    if let Some(max) = total_metrics.max_iteration {
        println!("Max iteration:           {:?}", max);
    }

    println!();
    println!(
        "Total address batches processed: {}",
        total_metrics.total_append_batches
    );

    if total_metrics.iterations > 0 {
        let avg_phase1 = total_metrics.phase1_total / total_metrics.iterations as u32;
        let avg_phase2 = total_metrics.phase2_total / total_metrics.iterations as u32;
        let avg_phase3 = total_metrics.phase3_total / total_metrics.iterations as u32;

        println!();
        println!("Phase timing breakdown (total / avg per iteration):");
        println!(
            "  Phase 1 (prep):        {:?} / {:?}",
            total_metrics.phase1_total, avg_phase1
        );
        println!(
            "  Phase 2 (proof):       {:?} / {:?}",
            total_metrics.phase2_total, avg_phase2
        );
        println!(
            "  Phase 3 (submit):      {:?} / {:?}",
            total_metrics.phase3_total, avg_phase3
        );
        println!("  ─────────────────────────────────────────────────────");
        println!(
            "  Total (actual):        {:?} / {:?}",
            total_metrics.total_duration,
            total_metrics.avg_iteration_duration()
        );
        println!();
        println!("Note: Phase 2 and Phase 3 run concurrently (pipelined),");
        println!("      so total < sum of individual phases.");
    }

    println!("========================================\n");
}
