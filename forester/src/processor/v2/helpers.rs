use crate::processor::v2::common::clamp_to_u16;
use anyhow::anyhow;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_client::{
    indexer::{Indexer, QueueElementsV2Options},
    rpc::Rpc,
};
use light_compressed_account::Pubkey;
use light_client::indexer::AddressQueueData;
use std::collections::HashMap;
use crate::processor::v2::BatchContext;
use light_client::indexer::StateQueueData;

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

pub async fn fetch_onchain_state_root<R: Rpc>(context: &BatchContext<R>) -> crate::Result<[u8; 32]> {
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

pub async fn fetch_onchain_address_root<R: Rpc>(context: &BatchContext<R>) -> crate::Result<[u8; 32]> {
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

const INDEXER_FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
const PAGE_SIZE_BATCHES: u64 = 20;

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
    let mut initial_root = None;
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

        if initial_root.is_none() {
            initial_root = Some(page.initial_root);
            root_seq = page.root_seq;
        } else if page.initial_root != initial_root.unwrap() {
            tracing::warn!(
                "Page {} has different root ({:?} vs {:?}), skipping",
                page_idx,
                &page.initial_root[..4],
                &initial_root.unwrap()[..4]
            );
            continue;
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
        output_queue.as_ref().map(|oq| oq.leaf_indices.len()).unwrap_or(0),
        input_queue.as_ref().map(|iq| iq.leaf_indices.len()).unwrap_or(0)
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

    let res = match tokio::time::timeout(INDEXER_FETCH_TIMEOUT, fetch_future).await {
        Ok(result) => result?,
        Err(_) => {
            tracing::warn!(
                "fetch_address_batches timed out after {:?} for tree {}",
                INDEXER_FETCH_TIMEOUT,
                context.merkle_tree
            );
            return Err(anyhow::anyhow!(
                "Indexer fetch timed out after {:?} for address tree {}",
                INDEXER_FETCH_TIMEOUT,
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

pub async fn fetch_paginated_address_batches<R: Rpc>(
    context: &BatchContext<R>,
    total_elements: u64,
    zkp_batch_size: u64,
) -> crate::Result<Option<light_client::indexer::AddressQueueData>> {
    
    if total_elements == 0 {
        return Ok(None);
    }

    let page_size_elements = PAGE_SIZE_BATCHES * zkp_batch_size;
    if total_elements <= page_size_elements {
        return fetch_address_batches(context, None, total_elements, zkp_batch_size).await;
    }

    let num_pages = total_elements.div_ceil(page_size_elements) as usize;
    tracing::debug!(
        "Parallel address fetch: {} elements ({} batches) in {} pages of {} batches each",
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
            fetch_address_batches(&ctx, page_offset, page_size, zkp_batch_size).await
        });

        offset += page_size;
    }

    let results = futures::future::join_all(fetch_futures).await;

    let mut initial_root = None;
    let mut start_index = 0u64;
    let mut root_seq = 0u64;
    let mut subtrees: Option<Vec<[u8; 32]>> = None;

    let mut addresses: Vec<[u8; 32]> = Vec::with_capacity(total_elements as usize);
    let mut low_element_values: Vec<[u8; 32]> = Vec::with_capacity(total_elements as usize);
    let mut low_element_next_values: Vec<[u8; 32]> = Vec::with_capacity(total_elements as usize);
    let mut low_element_indices: Vec<u64> = Vec::with_capacity(total_elements as usize);
    let mut low_element_next_indices: Vec<u64> = Vec::with_capacity(total_elements as usize);
    let mut leaves_hash_chains: Vec<[u8; 32]> = Vec::with_capacity(num_pages * 2); // heuristic
    let mut nodes_map: HashMap<u64, [u8; 32]> = HashMap::with_capacity(total_elements as usize);

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

        if initial_root.is_none() {
            initial_root = Some(page.initial_root);
            start_index = page.start_index;
            root_seq = page.root_seq;
            subtrees = Some(page.subtrees.clone());
        } else if page.initial_root != initial_root.unwrap() {
            tracing::warn!(
                "Address page {} has different root ({:?} vs {:?}), skipping",
                page_idx,
                &page.initial_root[..4],
                &initial_root.unwrap()[..4]
            );
            continue;
        }

        addresses.extend(page.addresses);
        low_element_values.extend(page.low_element_values);
        low_element_next_values.extend(page.low_element_next_values);
        low_element_indices.extend(page.low_element_indices);
        low_element_next_indices.extend(page.low_element_next_indices);
        leaves_hash_chains.extend(page.leaves_hash_chains);

        for (&idx, &hash) in page.nodes.iter().zip(page.node_hashes.iter()) {
            nodes_map.entry(idx).or_insert(hash);
        }
    }

    let initial_root = match initial_root {
        Some(root) => root,
        None => return Ok(None),
    };

    let subtrees = subtrees.ok_or_else(|| anyhow::anyhow!("No subtrees found in address queue data"))?;

    let mut nodes_vec: Vec<_> = nodes_map.into_iter().collect();
    nodes_vec.sort_by_key(|(idx, _)| *idx);
    let (nodes, node_hashes): (Vec<_>, Vec<_>) = nodes_vec.into_iter().unzip();

    tracing::debug!(
        "Parallel address fetch complete: {} addresses, {} nodes, {} leaves_hash_chains",
        addresses.len(),
        nodes.len(),
        leaves_hash_chains.len()
    );

    Ok(Some(AddressQueueData {
        addresses,
        low_element_values,
        low_element_next_values,
        low_element_indices,
        low_element_next_indices,
        subtrees,
        leaves_hash_chains,
        initial_root,
        start_index,
        root_seq,
        nodes,
        node_hashes,
    }))
}
