//! Indexer batch fetching: paginated state queue and address queue data retrieval.

use std::collections::HashMap;

use light_client::{
    indexer::{AddressQueueData, Indexer, QueueElementsV2Options, StateQueueData},
    rpc::Rpc,
};

use super::BatchContext;
use crate::processor::v2::common::clamp_to_u16;

const INDEXER_FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
const ADDRESS_INDEXER_FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
const PAGE_SIZE_BATCHES: u64 = 5;
pub(super) const ADDRESS_PAGE_SIZE_BATCHES: u64 = 5;

pub async fn fetch_paginated_batches<R: Rpc>(
    context: &BatchContext<R>,
    total_elements: u64,
    zkp_batch_size: u64,
) -> crate::Result<Option<StateQueueData>> {
    if zkp_batch_size == 0 {
        return Err(anyhow::anyhow!("zkp_batch_size cannot be zero"));
    }
    if total_elements == 0 {
        return Ok(None);
    }

    let page_size_elements = PAGE_SIZE_BATCHES * zkp_batch_size;
    if total_elements <= page_size_elements {
        tracing::debug!(
            "fetch_paginated_batches: single page fetch with start_index=None, total_elements={}, page_size={}",
            total_elements, page_size_elements
        );
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

    // Fetch first page with start_index=None to discover the actual first_queue_index
    // (queue may have been pruned, so indices don't start at 0)
    let first_page = fetch_batches(context, None, None, page_size_elements, zkp_batch_size).await?;

    let Some(first_page_data) = first_page else {
        return Ok(None);
    };

    // Get the actual starting indices from the first page response
    // IMPORTANT: Only use first_queue_index if the queue actually has elements.
    // When queue is empty, photon returns default first_queue_index=0, which would
    // cause subsequent pages to request start_index=2500 even though the actual
    // queue might start at 149500 (if elements arrive between requests).
    let output_first_index: Option<u64> = first_page_data
        .output_queue
        .as_ref()
        .filter(|oq| !oq.leaf_indices.is_empty())
        .map(|oq| oq.first_queue_index);
    let input_first_index: Option<u64> = first_page_data
        .input_queue
        .as_ref()
        .filter(|iq| !iq.leaf_indices.is_empty())
        .map(|iq| iq.first_queue_index);

    tracing::debug!(
        "First page fetched: output_first_index={:?}, input_first_index={:?}",
        output_first_index,
        input_first_index
    );

    // If only one page needed, return the first page result
    if num_pages == 1 {
        return Ok(Some(first_page_data));
    }

    // Fetch remaining pages in parallel with offsets relative to first_queue_index
    // Only request queues for which we have valid first_queue_index from the first page
    let mut fetch_futures = Vec::with_capacity(num_pages - 1);
    let mut offset = page_size_elements;

    for _page_idx in 1..num_pages {
        let page_size = (total_elements - offset).min(page_size_elements);
        // Only use Some(index) for queues we actually got data for in the first page
        // If first page had no data for a queue, we don't know its first_queue_index
        let output_start = output_first_index.map(|idx| idx + offset);
        let input_start = input_first_index.map(|idx| idx + offset);

        let ctx = context.clone();

        fetch_futures.push(async move {
            fetch_batches(&ctx, output_start, input_start, page_size, zkp_batch_size).await
        });

        offset += page_size;
    }

    let results = futures::future::join_all(fetch_futures).await;

    // Initialize with first page data
    let initial_root = first_page_data.initial_root;
    let root_seq = first_page_data.root_seq;
    let mut nodes_map: HashMap<u64, [u8; 32]> = HashMap::new();
    for (&idx, &hash) in first_page_data
        .nodes
        .iter()
        .zip(first_page_data.node_hashes.iter())
    {
        nodes_map.insert(idx, hash);
    }
    let mut output_queue = first_page_data.output_queue;
    let mut input_queue = first_page_data.input_queue;

    // Merge remaining pages
    for (page_idx, result) in results.into_iter().enumerate() {
        let page = match result? {
            Some(data) => data,
            None => continue,
        };

        if page.initial_root != initial_root {
            tracing::warn!(
                "Page {} has different root ({:?} vs {:?}), stopping merge",
                page_idx + 1,
                &page.initial_root[..4],
                &initial_root[..4]
            );
            break;
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
) -> crate::Result<Option<StateQueueData>> {
    tracing::debug!(
        "fetch_batches: tree={}, output_start={:?}, input_start={:?}, fetch_len={}, zkp_batch_size={}",
        context.merkle_tree, output_start_index, input_start_index, fetch_len, zkp_batch_size
    );

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
) -> crate::Result<Option<AddressQueueData>> {
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
