use crate::processor::v2::{
    common::{batch_range, get_leaves_hashchain},
    helpers::{fetch_address_batches, fetch_address_zkp_batch_size, fetch_onchain_address_root},
    proof_worker::ProofInput,
    strategy::{CircuitType, QueueData, TreeStrategy},
    BatchContext, QueueWork,
};
use anyhow::anyhow;
use async_trait::async_trait;
use forester_utils::{address_staging_tree::AddressStagingTree, utils::wait_for_indexer};
use light_batched_merkle_tree::constants::DEFAULT_BATCH_ADDRESS_TREE_HEIGHT;
use light_client::rpc::Rpc;
use tracing::{debug, info, warn};
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

        // Retry loop: fetch from indexer, validate root, wait if mismatch
        const MAX_ROOT_RETRIES: u32 = 3;
        // let mut address_queue = None;

        // for attempt in 0..MAX_ROOT_RETRIES {
        let address_queue =
            match fetch_address_batches(context, None, fetch_len, zkp_batch_size).await? {
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

        // Validate indexer root matches on-chain root before generating proofs
        // let onchain_root = fetch_onchain_address_root(context).await?;
        // if aq.initial_root == onchain_root {
        //     address_queue = Some(aq);
        //     break;
        // }

        //     warn!(
        //     "Indexer root mismatch for address tree {} (attempt {}/{}): indexer={}, onchain={}. Waiting for indexer...",
        //     context.merkle_tree,
        //     attempt + 1,
        //     MAX_ROOT_RETRIES,
        //     bs58::encode(&aq.initial_root).into_string(),
        //     bs58::encode(&onchain_root).into_string()
        // );

        // Wait for indexer to catch up
        //     let rpc = context.rpc_pool.get_connection().await?;
        //     if let Err(e) = wait_for_indexer(&*rpc).await {
        //         warn!("wait_for_indexer failed: {}", e);
        //     }
        // }

        // let address_queue = match address_queue {
        //     Some(aq) => aq,
        //     None => {
        //         warn!(
        //             "Indexer root still mismatched after {} retries for address tree {}. Skipping.",
        //             MAX_ROOT_RETRIES, context.merkle_tree
        //         );
        //         return Ok(None);
        //     }
        // };

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

        let subtrees_frontier: Option<[[u8; 32]; DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize]> =
            if address_queue.subtrees.len() == DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize {
                Some(
                    address_queue
                        .subtrees
                        .clone()
                        .try_into()
                        .map_err(|_| anyhow!("Failed to convert subtrees to array"))?,
                )
            } else {
                None
            };

        // Move CPU-bound tree initialization to blocking thread pool
        // to avoid blocking the async executor
        let nodes = address_queue.nodes.clone();
        let node_hashes = address_queue.node_hashes.clone();
        let staging_tree = tokio::task::spawn_blocking(move || {
            let start = std::time::Instant::now();
            let tree = if !nodes.is_empty() {
                AddressStagingTree::from_nodes(
                    &nodes,
                    &node_hashes,
                    initial_root,
                    start_index as usize,
                    subtrees_frontier,
                )
            } else if let Some(frontier) = subtrees_frontier {
                AddressStagingTree::from_nodes(
                    &[],
                    &[],
                    initial_root,
                    start_index as usize,
                    Some(frontier),
                )
            } else {
                Ok(AddressStagingTree::new(initial_root, start_index as usize))
            };
            info!(
                "AddressStagingTree init took {:?}, nodes={}, start_index={}",
                start.elapsed(),
                nodes.len(),
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
        batch_idx: usize,
        start: usize,
        zkp_batch_size: u64,
    ) -> crate::Result<(ProofInput, [u8; 32])> {
        let address_queue = &queue_data.address_queue;
        let range = batch_range(zkp_batch_size, address_queue.addresses.len(), start);
        let addresses = &address_queue.addresses[range.clone()];
        let zkp_batch_size_actual = addresses.len();

        let low_element_values = &address_queue.low_element_values[range.clone()];
        let low_element_next_values = &address_queue.low_element_next_values[range.clone()];
        let low_element_indices = &address_queue.low_element_indices[range.clone()];
        let low_element_next_indices = &address_queue.low_element_next_indices[range.clone()];
        let low_element_proofs = &address_queue.low_element_proofs[range];

        let leaves_hashchain = get_leaves_hashchain(&address_queue.leaves_hash_chains, batch_idx)?;

        let result = queue_data
            .staging_tree
            .process_batch(
                addresses,
                low_element_values,
                low_element_next_values,
                &low_element_indices,
                &low_element_next_indices,
                low_element_proofs,
                leaves_hashchain,
                zkp_batch_size_actual,
            )
            .map_err(|e| anyhow!("Failed to process address batch: {}", e))?;
        let new_root = result.new_root;
        Ok((ProofInput::AddressAppend(result.circuit_inputs), new_root))
    }
}
