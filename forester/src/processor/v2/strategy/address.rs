use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use forester_utils::address_staging_tree::AddressStagingTree;
use light_batched_merkle_tree::constants::DEFAULT_BATCH_ADDRESS_TREE_HEIGHT;
use light_client::rpc::Rpc;
use tracing::{debug, info};

use crate::processor::v2::{
    common::get_leaves_hashchain,
    helpers::{
        fetch_address_zkp_batch_size, fetch_onchain_address_root, fetch_streaming_address_batches,
        StreamingAddressQueue,
    },
    proof_worker::ProofInput,
    strategy::{CircuitType, QueueData, TreeStrategy},
    BatchContext, QueueWork,
};
use light_compressed_account::QueueType;

#[derive(Debug, Clone)]
pub struct AddressTreeStrategy;

pub struct AddressQueueData {
    pub staging_tree: AddressStagingTree,
    pub streaming_queue: Arc<StreamingAddressQueue>,
    pub data_start_index: u64,
    pub zkp_batch_size: usize,
}

impl AddressQueueData {
    pub fn check_alignment(&self) -> Result<usize, AddressAlignmentError> {
        let tree_next = self.staging_tree.next_index() as u64;
        let data_start = self.data_start_index;

        if data_start > tree_next {
            // Tree is stale - indexer has more elements than we know about
            Err(AddressAlignmentError::TreeStale {
                tree_next_index: tree_next,
                data_start_index: data_start,
            })
        } else if data_start == tree_next {
            // Perfect alignment
            Ok(0)
        } else {
            // Overlap - we've already processed some elements
            let overlap = (tree_next - data_start) as usize;
            Ok(overlap)
        }
    }

    /// Get the batch index to start processing from, accounting for overlap.
    /// Returns None if tree is stale.
    pub fn first_processable_batch(&self) -> Option<usize> {
        match self.check_alignment() {
            Ok(overlap) => {
                let batch_idx = overlap / self.zkp_batch_size;
                Some(batch_idx)
            }
            Err(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AddressAlignmentError {
    TreeStale {
        tree_next_index: u64,
        data_start_index: u64,
    },
}

impl std::fmt::Display for AddressAlignmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddressAlignmentError::TreeStale {
                tree_next_index,
                data_start_index,
            } => write!(
                f,
                "Address staging tree is stale: tree_next_index={}, data_start_index={}",
                tree_next_index, data_start_index
            ),
        }
    }
}

impl std::error::Error for AddressAlignmentError {}

impl std::fmt::Debug for AddressQueueData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddressQueueData")
            .field("staging_tree", &self.staging_tree)
            .field("data_start_index", &self.data_start_index)
            .field(
                "available_batches",
                &self.streaming_queue.available_batches(),
            )
            .field("alignment", &self.check_alignment())
            .finish()
    }
}


#[async_trait]
impl<R: Rpc> TreeStrategy<R> for AddressTreeStrategy {
    type StagingTree = AddressQueueData;

    fn name(&self) -> &'static str {
        "Address"
    }

    fn circuit_type(&self, _queue_data: &Self::StagingTree) -> CircuitType {
        CircuitType::AddressAppend
    }

    fn queue_type() -> QueueType {
        QueueType::AddressV2
    }

    async fn fetch_zkp_batch_size(&self, context: &BatchContext<R>) -> crate::Result<u64> {
        fetch_address_zkp_batch_size(context).await
    }

    async fn fetch_onchain_root(&self, context: &BatchContext<R>) -> crate::Result<[u8; 32]> {
        fetch_onchain_address_root(context).await
    }

    async fn fetch_queue_data(
        &self,
        context: &BatchContext<R>,
        _queue_work: &QueueWork,
        max_batches: usize,
        zkp_batch_size: u64,
    ) -> crate::Result<Option<QueueData<Self::StagingTree>>> {
        let zkp_batch_size_usize = zkp_batch_size as usize;
        let total_needed = max_batches.saturating_mul(zkp_batch_size_usize);
        let fetch_len = total_needed as u64;

        let streaming_queue =
            match fetch_streaming_address_batches(context, fetch_len, zkp_batch_size).await? {
                Some(sq) => Arc::new(sq),
                None => {
                    debug!("No address queue data available");
                    return Ok(None);
                }
            };

        let subtrees = streaming_queue.subtrees();
        if subtrees.is_empty() {
            return Err(anyhow!("Address queue missing subtrees data"));
        }

        let initial_batches = streaming_queue.available_batches();
        if initial_batches == 0 {
            debug!(
                "Not enough addresses for a complete batch in first page, need {}",
                zkp_batch_size_usize
            );
            return Ok(None);
        }

        let initial_root = streaming_queue.initial_root();
        let start_index = streaming_queue.start_index();

        let subtrees_arr: [[u8; 32]; DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize] =
            subtrees.try_into().map_err(|v: Vec<[u8; 32]>| {
                anyhow!(
                    "Subtrees length mismatch: expected {}, got {}",
                    DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
                    v.len()
                )
            })?;

        let staging_tree = tokio::task::spawn_blocking(move || {
            let start = std::time::Instant::now();
            let tree = AddressStagingTree::new(subtrees_arr, initial_root, start_index as usize);
            info!(
                "AddressStagingTree init took {:?}, start_index={}",
                start.elapsed(),
                start_index
            );
            tree
        })
        .await
        .map_err(|e| anyhow!("spawn_blocking join error: {}", e))??;

        let num_batches = initial_batches.min(max_batches);

        info!(
            "Address queue ready: {} batches available, processing {} (streaming in background), start_index={}",
            initial_batches, num_batches, start_index
        );

        Ok(Some(QueueData {
            staging_tree: AddressQueueData {
                staging_tree,
                streaming_queue,
                data_start_index: start_index,
                zkp_batch_size: zkp_batch_size as usize,
            },
            initial_root,
            num_batches,
        }))
    }

    fn build_proof_job(
        &self,
        queue_data: &mut Self::StagingTree,
        _batch_idx: usize,
        start: usize,
        zkp_batch_size: u64,
        epoch: u64,
        tree: &str,
    ) -> crate::Result<Option<(ProofInput, [u8; 32])>> {
        let zkp_batch_size_usize = zkp_batch_size as usize;

        let alignment = queue_data.check_alignment();
        let tree_next_index = queue_data.staging_tree.next_index();
        let data_start = queue_data.data_start_index as usize;

        let absolute_index = data_start + start;

        match &alignment {
            Err(AddressAlignmentError::TreeStale { .. }) => {
                return Err(anyhow!(
                    "Address staging tree is stale: tree_next_index={}, data_start_index={}. Need to invalidate cache.",
                    tree_next_index, data_start
                ));
            }
            Ok(overlap) if *overlap > 0 => {
                if absolute_index + zkp_batch_size_usize <= tree_next_index {
                    tracing::debug!(
                        "Skipping address batch (overlap): absolute_index={}, tree_next_index={}",
                        absolute_index,
                        tree_next_index
                    );
                    return Ok(None);
                } else if absolute_index < tree_next_index {
                    tracing::debug!(
                        "Skipping address batch (partial overlap): absolute_index={}, tree_next_index={}, batch_size={}",
                        absolute_index, tree_next_index, zkp_batch_size_usize
                    );
                    return Ok(None);
                }
            }
            _ => {
                // no overlap for this batch
            }
        }

        let batch_end = start + zkp_batch_size_usize;

        let batch_data = queue_data
            .streaming_queue
            .get_batch_data(start, batch_end)
            .ok_or_else(|| {
                anyhow!(
                    "Batch data not available: start={}, end={}, available={}",
                    start,
                    batch_end,
                    queue_data.streaming_queue.available_batches() * zkp_batch_size_usize
                )
            })?;

        let addresses = &batch_data.addresses;
        let zkp_batch_size_actual = addresses.len();

        if zkp_batch_size_actual == 0 {
            return Err(anyhow!("Empty batch at start={}", start));
        }

        let low_element_values = &batch_data.low_element_values;
        let low_element_next_values = &batch_data.low_element_next_values;
        let low_element_indices = &batch_data.low_element_indices;
        let low_element_next_indices = &batch_data.low_element_next_indices;

        // Get proofs from the streaming queue's data
        let low_element_proofs: Vec<Vec<[u8; 32]>> = {
            let data = queue_data.streaming_queue.data.lock().unwrap();
            (start..start + zkp_batch_size_actual)
                .map(|i| data.reconstruct_proof(i, DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as u8))
                .collect::<Result<Vec<_>, _>>()?
        };

        let hashchain_idx = start / zkp_batch_size_usize;

        // Get leaves_hash_chains from the streaming queue
        let leaves_hashchain = {
            let data = queue_data.streaming_queue.data.lock().unwrap();
            get_leaves_hashchain(&data.leaves_hash_chains, hashchain_idx)?
        };

        let tree_batch = tree_next_index / zkp_batch_size_usize;

        tracing::debug!(
            "Address build_proof_job: start={}, absolute_index={}, hashchain_idx={}, batch_size={}, tree_next_index={}, tree_batch={}, streaming_complete={}",
            start, absolute_index, hashchain_idx, zkp_batch_size_actual,
            tree_next_index, tree_batch,
            queue_data.streaming_queue.is_complete()
        );

        let result = queue_data
            .staging_tree
            .process_batch(
                addresses,
                low_element_values,
                low_element_next_values,
                low_element_indices,
                low_element_next_indices,
                &low_element_proofs,
                leaves_hashchain,
                zkp_batch_size_actual,
                epoch,
                tree,
            )
            .map_err(|e| anyhow!("Failed to process address batch: {}", e))?;
        let new_root = result.new_root;
        Ok(Some((
            ProofInput::AddressAppend(result.circuit_inputs),
            new_root,
        )))
    }

    fn available_batches(&self, queue_data: &Self::StagingTree, _zkp_batch_size: u64) -> usize {
        queue_data.streaming_queue.available_batches()
    }
}
