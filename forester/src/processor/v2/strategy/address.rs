use crate::processor::v2::{
    common::{batch_range, get_leaves_hashchain},
    helpers::{fetch_address_zkp_batch_size, fetch_onchain_address_root, fetch_paginated_address_batches},
    proof_worker::ProofInput,
    strategy::{CircuitType, QueueData, TreeStrategy},
    BatchContext, QueueWork,
};
use anyhow::anyhow;
use async_trait::async_trait;
use forester_utils::address_staging_tree::AddressStagingTree;
use light_batched_merkle_tree::constants::DEFAULT_BATCH_ADDRESS_TREE_HEIGHT;
use light_client::rpc::Rpc;
use tracing::{debug, info};
#[derive(Debug, Clone)]
pub struct AddressTreeStrategy;

#[derive(Debug)]
pub struct AddressQueueData {
    pub staging_tree: AddressStagingTree,
    pub address_queue: light_client::indexer::AddressQueueDataV2,
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

        // Use paginated fetch for parallel page fetching (similar to state trees)
        let address_queue =
            match fetch_paginated_address_batches(context, fetch_len, zkp_batch_size).await? {
                Some(aq) if !aq.addresses.is_empty() => aq,
                Some(_) => {
                    debug!("Address queue is empty");
                    return Ok(None);
                }
                None => {
                    debug!("No address queue data available");
                    return Ok(None);
                }
            };

        if address_queue.subtrees.is_empty() {
            return Err(anyhow!("Address queue missing subtrees data"));
        }

        let available = address_queue.addresses.len();
        let num_batches = (available / zkp_batch_size_usize).min(max_batches);

        if num_batches == 0 {
            debug!(
                "Not enough addresses for a complete batch: have {}, need {}",
                available, zkp_batch_size_usize
            );
            return Ok(None);
        }

        if address_queue.leaves_hash_chains.len() < num_batches {
            return Err(anyhow!(
                "Insufficient leaves_hash_chains: have {}, need {}",
                address_queue.leaves_hash_chains.len(),
                num_batches
            ));
        }

        let initial_root = address_queue.initial_root;
        let start_index = address_queue.start_index;

        // Convert subtrees to array (required for sparse tree)
        let subtrees: [[u8; 32]; DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize] = address_queue
            .subtrees
            .clone()
            .try_into()
            .map_err(|v: Vec<[u8; 32]>| {
                anyhow!(
                    "Subtrees length mismatch: expected {}, got {}",
                    DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
                    v.len()
                )
            })?;

        // Initialize the staging tree with subtrees (sparse tree approach)
        let staging_tree = tokio::task::spawn_blocking(move || {
            let start = std::time::Instant::now();
            let tree = AddressStagingTree::new(subtrees, initial_root, start_index as usize);
            info!(
                "AddressStagingTree init took {:?}, start_index={}",
                start.elapsed(),
                start_index
            );
            tree
        })
        .await
        .map_err(|e| anyhow!("spawn_blocking join error: {}", e))??;

        Ok(Some(QueueData {
            staging_tree: AddressQueueData {
                staging_tree,
                address_queue,
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
    ) -> crate::Result<(ProofInput, [u8; 32])> {
        let address_queue = &queue_data.address_queue;
        let range = batch_range(zkp_batch_size, address_queue.addresses.len(), start);
        let addresses = &address_queue.addresses[range.clone()];
        let zkp_batch_size_actual = addresses.len();

        let low_element_values = &address_queue.low_element_values[range.clone()];
        let low_element_next_values = &address_queue.low_element_next_values[range.clone()];
        let low_element_indices = &address_queue.low_element_indices[range.clone()];
        let low_element_next_indices = &address_queue.low_element_next_indices[range.clone()];

        // Reconstruct proofs from deduplicated nodes
        let low_element_proofs: Vec<Vec<[u8; 32]>> = range
            .clone()
            .map(|i| {
                address_queue.reconstruct_proof(i, DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as u8)
            })
            .collect();

        // Use start-based index like state strategy does, not batch_idx
        // This handles cached state continuation correctly
        let hashchain_idx = start / zkp_batch_size as usize;

        // Also compute the expected batch from start_index (tree position)
        let tree_batch = queue_data.staging_tree.next_index() / zkp_batch_size as usize;

        tracing::debug!(
            "Address build_proof_job: start={}, hashchain_idx={}, addresses_len={}, leaves_hash_chains_len={}, zkp_batch_size={}, tree_next_index={}, tree_batch={}",
            start, hashchain_idx, address_queue.addresses.len(), address_queue.leaves_hash_chains.len(), zkp_batch_size,
            queue_data.staging_tree.next_index(), tree_batch
        );

        // Log first 3 addresses for debugging
        for (i, addr) in addresses.iter().take(3).enumerate() {
            tracing::debug!("  addresses[{}] = {:?}[..8]", start + i, &addr[..8]);
        }

        let leaves_hashchain = get_leaves_hashchain(&address_queue.leaves_hash_chains, hashchain_idx)?;

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
        Ok((ProofInput::AddressAppend(result.circuit_inputs), new_root))
    }
}
