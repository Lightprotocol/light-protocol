use std::{
    collections::HashMap,
    sync::{Arc, Condvar, Mutex},
};

use anyhow::anyhow;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_client::{
    indexer::{AddressQueueData, Indexer, QueueElementsV2Options, StateQueueData},
    rpc::Rpc,
};
use light_compressed_account::Pubkey;

use crate::processor::v2::{common::clamp_to_u16, BatchContext};

pub async fn fetch_zkp_batch_size<R: Rpc>(context: &BatchContext<R>) -> crate::Result<u64> {
    let rpc = context.rpc_pool.get_connection().await?;
    let mut account = rpc
        .get_account(context.merkle_tree)
        .await?
        .ok_or_else(|| anyhow!("Merkle tree account not found"))?;

    let tree = BatchedMerkleTreeAccount::state_from_bytes(
        account.data.as_mut_slice(),
        &context.merkle_tree.into(),
    )?;

    let batch_index = tree.queue_batches.pending_batch_index;
    let batch = tree
        .queue_batches
        .batches
        .get(batch_index as usize)
        .ok_or_else(|| anyhow!("Batch not found"))?;

    Ok(batch.zkp_batch_size)
}

pub async fn fetch_onchain_state_root<R: Rpc>(
    context: &BatchContext<R>,
) -> crate::Result<[u8; 32]> {
    let rpc = context.rpc_pool.get_connection().await?;
    let mut account = rpc
        .get_account(context.merkle_tree)
        .await?
        .ok_or_else(|| anyhow!("Merkle tree account not found"))?;

    let tree = BatchedMerkleTreeAccount::state_from_bytes(
        account.data.as_mut_slice(),
        &context.merkle_tree.into(),
    )?;

    // Get the current root (last entry in root_history)
    let root = tree
        .root_history
        .last()
        .copied()
        .ok_or_else(|| anyhow!("Root history is empty"))?;

    Ok(root)
}

pub async fn fetch_address_zkp_batch_size<R: Rpc>(context: &BatchContext<R>) -> crate::Result<u64> {
    let rpc = context.rpc_pool.get_connection().await?;
    let mut account = rpc
        .get_account(context.merkle_tree)
        .await?
        .ok_or_else(|| anyhow!("Merkle tree account not found"))?;

    let merkle_tree_pubkey = Pubkey::from(context.merkle_tree.to_bytes());
    let tree = BatchedMerkleTreeAccount::address_from_bytes(&mut account.data, &merkle_tree_pubkey)
        .map_err(|e| anyhow!("Failed to deserialize address tree: {}", e))?;

    let batch_index = tree.queue_batches.pending_batch_index;
    let batch = tree
        .queue_batches
        .batches
        .get(batch_index as usize)
        .ok_or_else(|| anyhow!("Batch not found"))?;

    Ok(batch.zkp_batch_size)
}

pub async fn fetch_onchain_address_root<R: Rpc>(
    context: &BatchContext<R>,
) -> crate::Result<[u8; 32]> {
    let rpc = context.rpc_pool.get_connection().await?;
    let mut account = rpc
        .get_account(context.merkle_tree)
        .await?
        .ok_or_else(|| anyhow!("Merkle tree account not found"))?;

    let merkle_tree_pubkey = Pubkey::from(context.merkle_tree.to_bytes());
    let tree = BatchedMerkleTreeAccount::address_from_bytes(&mut account.data, &merkle_tree_pubkey)
        .map_err(|e| anyhow!("Failed to deserialize address tree: {}", e))?;

    let root = tree
        .root_history
        .last()
        .copied()
        .ok_or_else(|| anyhow!("Root history is empty"))?;

    Ok(root)
}

const INDEXER_FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
const ADDRESS_INDEXER_FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
const PAGE_SIZE_BATCHES: u64 = 20;
const ADDRESS_PAGE_SIZE_BATCHES: u64 = 5;

pub async fn fetch_paginated_batches<R: Rpc>(
    context: &BatchContext<R>,
    total_elements: u64,
    zkp_batch_size: u64,
) -> crate::Result<Option<light_client::indexer::StateQueueData>> {
    if total_elements == 0 {
        return Ok(None);
    }

    let page_size_elements = PAGE_SIZE_BATCHES * zkp_batch_size;
    if total_elements <= page_size_elements {
        return fetch_batches(context, None, None, total_elements, zkp_batch_size).await;
    }

    let num_pages = total_elements.div_ceil(page_size_elements) as usize;
    tracing::debug!(
        "Parallel fetch: {} elements ({} batches) in {} pages of {} batches each",
        total_elements,
        total_elements / zkp_batch_size,
        num_pages,
        PAGE_SIZE_BATCHES
    );

    let mut fetch_futures = Vec::with_capacity(num_pages);
    let mut offset = 0u64;

    for page_idx in 0..num_pages {
        let page_size = (total_elements - offset).min(page_size_elements);
        let page_offset = if page_idx == 0 { None } else { Some(offset) };

        let ctx = context.clone();

        fetch_futures.push(async move {
            fetch_batches(&ctx, page_offset, page_offset, page_size, zkp_batch_size).await
        });

        offset += page_size;
    }

    let results = futures::future::join_all(fetch_futures).await;
    let mut initial_root: Option<[u8; 32]> = None;
    let mut root_seq = 0u64;
    let mut nodes_map: HashMap<u64, [u8; 32]> = HashMap::new();
    let mut output_queue: Option<light_client::indexer::OutputQueueData> = None;
    let mut input_queue: Option<light_client::indexer::InputQueueData> = None;

    for (page_idx, result) in results.into_iter().enumerate() {
        let page = match result? {
            Some(data) => data,
            None => {
                if page_idx == 0 {
                    return Ok(None);
                }
                continue;
            }
        };

        if let Some(root) = initial_root {
            if page.initial_root != root {
                tracing::warn!(
                    "Page {} has different root ({:?} vs {:?}), skipping",
                    page_idx,
                    &page.initial_root[..4],
                    &initial_root.unwrap()[..4]
                );
                continue;
            }
        } else {
            initial_root = Some(page.initial_root);
            root_seq = page.root_seq;
        }

        for (&idx, &hash) in page.nodes.iter().zip(page.node_hashes.iter()) {
            nodes_map.entry(idx).or_insert(hash);
        }

        if let Some(page_oq) = page.output_queue {
            if let Some(ref mut oq) = output_queue {
                oq.leaf_indices.extend(page_oq.leaf_indices);
                oq.account_hashes.extend(page_oq.account_hashes);
                oq.old_leaves.extend(page_oq.old_leaves);
                oq.leaves_hash_chains.extend(page_oq.leaves_hash_chains);
            } else {
                output_queue = Some(page_oq);
            }
        }

        if let Some(page_iq) = page.input_queue {
            if let Some(ref mut iq) = input_queue {
                iq.leaf_indices.extend(page_iq.leaf_indices);
                iq.account_hashes.extend(page_iq.account_hashes);
                iq.current_leaves.extend(page_iq.current_leaves);
                iq.tx_hashes.extend(page_iq.tx_hashes);
                iq.nullifiers.extend(page_iq.nullifiers);
                iq.leaves_hash_chains.extend(page_iq.leaves_hash_chains);
            } else {
                input_queue = Some(page_iq);
            }
        }
    }

    let initial_root = match initial_root {
        Some(root) => root,
        None => return Ok(None),
    };

    let mut nodes_vec: Vec<_> = nodes_map.into_iter().collect();
    nodes_vec.sort_by_key(|(idx, _)| *idx);
    let (nodes, node_hashes): (Vec<_>, Vec<_>) = nodes_vec.into_iter().unzip();

    tracing::debug!(
        "Parallel fetch complete: {} nodes, output={}, input={}",
        nodes.len(),
        output_queue
            .as_ref()
            .map(|oq| oq.leaf_indices.len())
            .unwrap_or(0),
        input_queue
            .as_ref()
            .map(|iq| iq.leaf_indices.len())
            .unwrap_or(0)
    );

    Ok(Some(StateQueueData {
        nodes,
        node_hashes,
        initial_root,
        root_seq,
        output_queue,
        input_queue,
    }))
}

pub async fn fetch_batches<R: Rpc>(
    context: &BatchContext<R>,
    output_start_index: Option<u64>,
    input_start_index: Option<u64>,
    fetch_len: u64,
    zkp_batch_size: u64,
) -> crate::Result<Option<light_client::indexer::StateQueueData>> {
    let fetch_len_u16 = clamp_to_u16(fetch_len, "fetch_len");
    let zkp_batch_size_u16 = clamp_to_u16(zkp_batch_size, "zkp_batch_size");

    let mut rpc = context.rpc_pool.get_connection().await?;
    let indexer = rpc.indexer_mut()?;
    let options = QueueElementsV2Options::default()
        .with_output_queue(output_start_index, Some(fetch_len_u16))
        .with_output_queue_batch_size(Some(zkp_batch_size_u16))
        .with_input_queue(input_start_index, Some(fetch_len_u16))
        .with_input_queue_batch_size(Some(zkp_batch_size_u16));

    let fetch_future = indexer.get_queue_elements(context.merkle_tree.to_bytes(), options, None);

    let res = match tokio::time::timeout(INDEXER_FETCH_TIMEOUT, fetch_future).await {
        Ok(result) => result?,
        Err(_) => {
            tracing::warn!(
                "fetch_batches timed out after {:?} for tree {}",
                INDEXER_FETCH_TIMEOUT,
                context.merkle_tree
            );
            return Err(anyhow::anyhow!(
                "Indexer fetch timed out after {:?} for state tree {}",
                INDEXER_FETCH_TIMEOUT,
                context.merkle_tree
            ));
        }
    };

    Ok(res.value.state_queue)
}

pub async fn fetch_address_batches<R: Rpc>(
    context: &BatchContext<R>,
    output_start_index: Option<u64>,
    fetch_len: u64,
    zkp_batch_size: u64,
) -> crate::Result<Option<light_client::indexer::AddressQueueData>> {
    let fetch_len_u16 = clamp_to_u16(fetch_len, "fetch_len");
    let zkp_batch_size_u16 = clamp_to_u16(zkp_batch_size, "zkp_batch_size");

    let mut rpc = context.rpc_pool.get_connection().await?;
    let indexer = rpc.indexer_mut()?;

    let options = QueueElementsV2Options::default()
        .with_address_queue(output_start_index, Some(fetch_len_u16))
        .with_address_queue_batch_size(Some(zkp_batch_size_u16));

    tracing::debug!(
        "fetch_address_batches: tree={}, start={:?}, len={}, zkp_batch_size={}",
        context.merkle_tree,
        output_start_index,
        fetch_len_u16,
        zkp_batch_size_u16
    );

    let fetch_future = indexer.get_queue_elements(context.merkle_tree.to_bytes(), options, None);

    let res = match tokio::time::timeout(ADDRESS_INDEXER_FETCH_TIMEOUT, fetch_future).await {
        Ok(result) => result?,
        Err(_) => {
            tracing::warn!(
                "fetch_address_batches timed out after {:?} for tree {}",
                ADDRESS_INDEXER_FETCH_TIMEOUT,
                context.merkle_tree
            );
            return Err(anyhow::anyhow!(
                "Indexer fetch timed out after {:?} for address tree {}",
                ADDRESS_INDEXER_FETCH_TIMEOUT,
                context.merkle_tree
            ));
        }
    };

    if let Some(ref aq) = res.value.address_queue {
        tracing::debug!(
            "fetch_address_batches response: address_queue present = true, addresses={}, subtrees={}, leaves_hash_chains={}, start_index={}",
            aq.addresses.len(),
            aq.subtrees.len(),
            aq.leaves_hash_chains.len(),
            aq.start_index
        );
    } else {
        tracing::debug!("fetch_address_batches response: address_queue present = false");
    }

    Ok(res.value.address_queue)
}

#[derive(Debug)]
pub struct StreamingAddressQueue {
    pub data: Arc<Mutex<AddressQueueData>>,
    available_elements: Arc<Mutex<usize>>,
    /// Condition variable to wait for more data
    data_ready: Arc<Condvar>,
    /// Whether all fetching is complete
    fetch_complete: Arc<Mutex<bool>>,
    /// Total elements expected
    #[allow(dead_code)]
    total_expected: usize,
    /// ZKP batch size
    zkp_batch_size: usize,
}

impl StreamingAddressQueue {
    pub fn wait_for_batch(&self, batch_end: usize) -> usize {
        let mut available = self.available_elements.lock().unwrap();
        while *available < batch_end {
            let complete = *self.fetch_complete.lock().unwrap();
            if complete {
                break;
            }
            available = self.data_ready.wait(available).unwrap();
        }
        *available
    }

    pub fn get_batch_data(&self, start: usize, end: usize) -> Option<BatchDataSlice> {
        let available = self.wait_for_batch(end);
        if start >= available {
            return None;
        }
        let actual_end = end.min(available);
        let data = self.data.lock().unwrap();
        Some(BatchDataSlice {
            addresses: data.addresses[start..actual_end].to_vec(),
            low_element_values: data.low_element_values[start..actual_end].to_vec(),
            low_element_next_values: data.low_element_next_values[start..actual_end].to_vec(),
            low_element_indices: data.low_element_indices[start..actual_end].to_vec(),
            low_element_next_indices: data.low_element_next_indices[start..actual_end].to_vec(),
        })
    }

    pub fn into_data(self) -> AddressQueueData {
        let mut complete = self.fetch_complete.lock().unwrap();
        while !*complete {
            complete = self.data_ready.wait_while(complete, |c| !*c).unwrap();
        }
        drop(complete);
        Arc::try_unwrap(self.data)
            .map(|mutex| mutex.into_inner().unwrap())
            .unwrap_or_else(|arc| arc.lock().unwrap().clone())
    }

    pub fn initial_root(&self) -> [u8; 32] {
        self.data.lock().unwrap().initial_root
    }

    pub fn start_index(&self) -> u64 {
        self.data.lock().unwrap().start_index
    }

    pub fn subtrees(&self) -> Vec<[u8; 32]> {
        self.data.lock().unwrap().subtrees.clone()
    }

    pub fn root_seq(&self) -> u64 {
        self.data.lock().unwrap().root_seq
    }

    pub fn available_batches(&self) -> usize {
        let available = *self.available_elements.lock().unwrap();
        available / self.zkp_batch_size
    }

    pub fn is_complete(&self) -> bool {
        *self.fetch_complete.lock().unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct BatchDataSlice {
    pub addresses: Vec<[u8; 32]>,
    pub low_element_values: Vec<[u8; 32]>,
    pub low_element_next_values: Vec<[u8; 32]>,
    pub low_element_indices: Vec<u64>,
    pub low_element_next_indices: Vec<u64>,
}

pub async fn fetch_streaming_address_batches<R: Rpc + 'static>(
    context: &BatchContext<R>,
    total_elements: u64,
    zkp_batch_size: u64,
) -> crate::Result<Option<StreamingAddressQueue>> {
    if total_elements == 0 {
        return Ok(None);
    }

    let page_size_elements = ADDRESS_PAGE_SIZE_BATCHES * zkp_batch_size;
    let num_pages = total_elements.div_ceil(page_size_elements) as usize;

    tracing::info!(
        "address fetch: {} elements ({} batches) in {} pages of {} batches each",
        total_elements,
        total_elements / zkp_batch_size,
        num_pages,
        ADDRESS_PAGE_SIZE_BATCHES
    );

    let first_page_size = page_size_elements.min(total_elements);
    let first_page =
        match fetch_address_batches(context, None, first_page_size, zkp_batch_size).await? {
            Some(data) if !data.addresses.is_empty() => data,
            _ => return Ok(None),
        };

    let initial_elements = first_page.addresses.len();
    let first_page_requested = first_page_size as usize;

    let queue_exhausted = initial_elements < first_page_requested;

    tracing::info!(
        "First page fetched: {} addresses ({} batches ready), root={:?}[..4], queue_exhausted={}",
        initial_elements,
        initial_elements / zkp_batch_size as usize,
        &first_page.initial_root[..4],
        queue_exhausted
    );

    let streaming = StreamingAddressQueue {
        data: Arc::new(Mutex::new(first_page)),
        available_elements: Arc::new(Mutex::new(initial_elements)),
        data_ready: Arc::new(Condvar::new()),
        fetch_complete: Arc::new(Mutex::new(num_pages == 1 || queue_exhausted)),
        total_expected: total_elements as usize,
        zkp_batch_size: zkp_batch_size as usize,
    };

    if num_pages == 1 || queue_exhausted {
        return Ok(Some(streaming));
    }

    let data = Arc::clone(&streaming.data);
    let available = Arc::clone(&streaming.available_elements);
    let ready = Arc::clone(&streaming.data_ready);
    let complete = Arc::clone(&streaming.fetch_complete);
    let ctx = context.clone();
    let initial_root = streaming.initial_root();

    tokio::spawn(async move {
        let mut offset = first_page_size;

        for page_idx in 1..num_pages {
            let page_size = (total_elements - offset).min(page_size_elements);

            tracing::debug!(
                "Fetching address page {}/{}: offset={}, size={}",
                page_idx + 1,
                num_pages,
                offset,
                page_size
            );

            match fetch_address_batches(&ctx, Some(offset), page_size, zkp_batch_size).await {
                Ok(Some(page)) => {
                    if page.initial_root != initial_root {
                        tracing::warn!(
                            "Address page {} has different root ({:?} vs {:?}), stopping fetch",
                            page_idx + 1,
                            &page.initial_root[..4],
                            &initial_root[..4]
                        );
                        break;
                    }

                    let page_elements = page.addresses.len();
                    let page_requested = page_size as usize;

                    {
                        let mut data_guard = data.lock().unwrap();
                        data_guard.addresses.extend(page.addresses);
                        data_guard
                            .low_element_values
                            .extend(page.low_element_values);
                        data_guard
                            .low_element_next_values
                            .extend(page.low_element_next_values);
                        data_guard
                            .low_element_indices
                            .extend(page.low_element_indices);
                        data_guard
                            .low_element_next_indices
                            .extend(page.low_element_next_indices);
                        data_guard
                            .leaves_hash_chains
                            .extend(page.leaves_hash_chains);
                        for (&idx, &hash) in page.nodes.iter().zip(page.node_hashes.iter()) {
                            if !data_guard.nodes.contains(&idx) {
                                data_guard.nodes.push(idx);
                                data_guard.node_hashes.push(hash);
                            }
                        }
                    }

                    {
                        let mut avail = available.lock().unwrap();
                        *avail += page_elements;
                        tracing::debug!(
                            "Page {} fetched: {} elements, total available: {} ({} batches)",
                            page_idx + 1,
                            page_elements,
                            *avail,
                            *avail / zkp_batch_size as usize
                        );
                    }
                    ready.notify_all();

                    if page_elements < page_requested {
                        tracing::debug!(
                            "Page {} returned fewer elements than requested ({} < {}), queue exhausted",
                            page_idx + 1, page_elements, page_requested
                        );
                        break;
                    }
                }
                Ok(None) => {
                    tracing::debug!("Page {} returned empty, stopping fetch", page_idx + 1);
                    break;
                }
                Err(e) => {
                    tracing::warn!("Error fetching page {}: {}", page_idx + 1, e);
                    break;
                }
            }

            offset += page_size;
        }

        *complete.lock().unwrap() = true;
        ready.notify_all();
        tracing::debug!("Background address fetch complete");
    });

    Ok(Some(streaming))
}

#[allow(dead_code)]
pub async fn fetch_paginated_address_batches<R: Rpc + 'static>(
    context: &BatchContext<R>,
    total_elements: u64,
    zkp_batch_size: u64,
) -> crate::Result<Option<AddressQueueData>> {
    match fetch_streaming_address_batches(context, total_elements, zkp_batch_size).await? {
        Some(streaming) => Ok(Some(streaming.into_data())),
        None => Ok(None),
    }
}
