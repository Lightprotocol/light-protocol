//! StreamingAddressQueue: background-fetched address queue with batch-level access.

use std::{
    collections::HashSet,
    sync::{Arc, Condvar, Mutex, MutexGuard},
};

use anyhow::anyhow;
use light_client::{indexer::AddressQueueData, rpc::Rpc};
use light_hasher::hash_chain::create_hash_chain_from_slice;

use super::{
    indexer_fetch::{fetch_address_batches, ADDRESS_PAGE_SIZE_BATCHES},
    BatchContext,
};
use crate::logging::should_emit_rate_limited_warning;

fn lock_recover<'a, T>(mutex: &'a Mutex<T>, name: &'static str) -> MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!("Poisoned mutex (recovering): {}", name);
            poisoned.into_inner()
        }
    }
}

#[derive(Debug, Clone)]
pub struct AddressBatchSnapshot<const HEIGHT: usize> {
    pub addresses: Vec<[u8; 32]>,
    pub low_element_values: Vec<[u8; 32]>,
    pub low_element_next_values: Vec<[u8; 32]>,
    pub low_element_indices: Vec<u64>,
    pub low_element_next_indices: Vec<u64>,
    pub low_element_proofs: Vec<[[u8; 32]; HEIGHT]>,
    pub leaves_hashchain: [u8; 32],
}

/// Streams address queue data by fetching pages in the background.
///
/// The first page is fetched synchronously, then subsequent pages are fetched
/// in a background task. Consumers can access data as it becomes available
/// without waiting for the entire fetch to complete.
#[derive(Debug)]
pub struct StreamingAddressQueue {
    /// The accumulated address queue data from all fetched pages.
    pub data: Arc<Mutex<AddressQueueData>>,

    /// Number of elements currently available for processing.
    available_elements: Arc<Mutex<usize>>,

    /// Signaled when new elements become available.
    data_ready: Arc<Condvar>,

    /// Whether the background fetch has completed (all pages fetched or error).
    fetch_complete: Arc<Mutex<bool>>,

    /// Signaled when background fetch completes.
    fetch_complete_condvar: Arc<Condvar>,

    /// Number of elements per ZKP batch, used for batch boundary calculations.
    zkp_batch_size: usize,
}

impl StreamingAddressQueue {
    /// Waits until at least `batch_end` elements are available or fetch completes.
    pub fn wait_for_batch(&self, batch_end: usize) -> usize {
        const POLL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
        let start = std::time::Instant::now();

        loop {
            let available = *lock_recover(
                &self.available_elements,
                "streaming_address_queue.available_elements",
            );
            if available >= batch_end {
                return available;
            }

            let complete = *lock_recover(
                &self.fetch_complete,
                "streaming_address_queue.fetch_complete",
            );
            if complete {
                return available;
            }

            if start.elapsed() > POLL_TIMEOUT {
                tracing::warn!(
                    "wait_for_batch timed out after {:?} waiting for {} elements (available: {})",
                    POLL_TIMEOUT,
                    batch_end,
                    available
                );
                return available;
            }

            let guard = lock_recover(
                &self.available_elements,
                "streaming_address_queue.available_elements",
            );
            let _ = self
                .data_ready
                .wait_timeout(guard, std::time::Duration::from_millis(50));
        }
    }

    pub fn get_batch_snapshot<const HEIGHT: usize>(
        &self,
        start: usize,
        end: usize,
        hashchain_idx: usize,
    ) -> crate::Result<Option<AddressBatchSnapshot<HEIGHT>>> {
        let available = self.wait_for_batch(end);
        if available < end || start >= end {
            return Ok(None);
        }
        let data = lock_recover(&self.data, "streaming_address_queue.data");

        let range = start..end;
        let (
            Some(addresses),
            Some(low_element_values),
            Some(low_element_next_values),
            Some(low_element_indices),
            Some(low_element_next_indices),
        ) = (
            data.addresses.get(range.clone()).map(<[_]>::to_vec),
            data.low_element_values
                .get(range.clone())
                .map(<[_]>::to_vec),
            data.low_element_next_values
                .get(range.clone())
                .map(<[_]>::to_vec),
            data.low_element_indices
                .get(range.clone())
                .map(<[_]>::to_vec),
            data.low_element_next_indices
                .get(range.clone())
                .map(<[_]>::to_vec),
        )
        else {
            return Ok(None);
        };

        let low_element_proofs = match data.reconstruct_proofs::<HEIGHT>(range) {
            Ok(proofs) => proofs,
            Err(error) => {
                if should_emit_rate_limited_warning(
                    "address_queue_proofs_not_ready",
                    std::time::Duration::from_secs(60),
                ) {
                    tracing::warn!(
                        ?error,
                        start,
                        end,
                        "address proof reconstruction not ready, retrying"
                    );
                }
                return Ok(None);
            }
        };

        let leaves_hashchain = match data.leaves_hash_chains.get(hashchain_idx).copied() {
            Some(hashchain) => hashchain,
            None => {
                tracing::debug!(
                    "Missing leaves_hash_chain for batch {} (available: {}), deriving from addresses",
                    hashchain_idx,
                    data.leaves_hash_chains.len()
                );
                create_hash_chain_from_slice(&addresses).map_err(|error| {
                    anyhow!(
                        "Failed to derive leaves_hash_chain for batch {} from {} addresses: {}",
                        hashchain_idx,
                        addresses.len(),
                        error
                    )
                })?
            }
        };

        Ok(Some(AddressBatchSnapshot {
            low_element_values,
            low_element_next_values,
            low_element_indices,
            low_element_next_indices,
            low_element_proofs,
            addresses,
            leaves_hashchain,
        }))
    }

    pub fn into_data(self) -> AddressQueueData {
        let mut complete = lock_recover(
            &self.fetch_complete,
            "streaming_address_queue.fetch_complete",
        );
        while !*complete {
            complete = match self.fetch_complete_condvar.wait_while(complete, |c| !*c) {
                Ok(guard) => guard,
                Err(poisoned) => {
                    tracing::warn!("Poisoned mutex while waiting (recovering): streaming_address_queue.fetch_complete");
                    poisoned.into_inner()
                }
            };
        }
        drop(complete);
        match Arc::try_unwrap(self.data) {
            Ok(mutex) => mutex.into_inner().unwrap_or_else(|poisoned| {
                tracing::warn!("Poisoned mutex during into_data (recovering)");
                poisoned.into_inner()
            }),
            Err(arc) => lock_recover(arc.as_ref(), "streaming_address_queue.data_clone").clone(),
        }
    }

    pub fn initial_root(&self) -> [u8; 32] {
        lock_recover(&self.data, "streaming_address_queue.data").initial_root
    }

    pub fn start_index(&self) -> u64 {
        lock_recover(&self.data, "streaming_address_queue.data").start_index
    }

    pub fn tree_next_insertion_index(&self) -> u64 {
        lock_recover(&self.data, "streaming_address_queue.data").tree_next_insertion_index
    }

    pub fn subtrees(&self) -> Vec<[u8; 32]> {
        lock_recover(&self.data, "streaming_address_queue.data")
            .subtrees
            .clone()
    }

    pub fn root_seq(&self) -> u64 {
        lock_recover(&self.data, "streaming_address_queue.data").root_seq
    }

    pub fn available_batches(&self) -> usize {
        debug_assert!(self.zkp_batch_size != 0, "zkp_batch_size must be non-zero");
        if self.zkp_batch_size == 0 {
            tracing::error!("zkp_batch_size is zero, returning 0 batches to avoid panic");
            return 0;
        }
        let available = *lock_recover(
            &self.available_elements,
            "streaming_address_queue.available_elements",
        );
        available / self.zkp_batch_size
    }

    pub fn is_complete(&self) -> bool {
        *lock_recover(
            &self.fetch_complete,
            "streaming_address_queue.fetch_complete",
        )
    }
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

    tracing::debug!(
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
        fetch_complete_condvar: Arc::new(Condvar::new()),
        zkp_batch_size: zkp_batch_size as usize,
    };

    if num_pages == 1 || queue_exhausted {
        return Ok(Some(streaming));
    }

    let data = Arc::clone(&streaming.data);
    let available = Arc::clone(&streaming.available_elements);
    let ready = Arc::clone(&streaming.data_ready);
    let complete = Arc::clone(&streaming.fetch_complete);
    let complete_condvar = Arc::clone(&streaming.fetch_complete_condvar);
    let ctx = context.clone();
    let initial_root = streaming.initial_root();
    let first_page_start_index = streaming.start_index();

    tokio::spawn(async move {
        let mut offset = first_page_size;

        for page_idx in 1..num_pages {
            let page_size = (total_elements - offset).min(page_size_elements);
            let absolute_start = Some(first_page_start_index + offset);

            tracing::debug!(
                "Fetching address page {}/{}: absolute_start={:?}, size={}",
                page_idx + 1,
                num_pages,
                absolute_start,
                page_size
            );

            match fetch_address_batches(&ctx, absolute_start, page_size, zkp_batch_size).await {
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
                        let mut data_guard =
                            lock_recover(data.as_ref(), "streaming_address_queue.data");
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
                        let mut seen: HashSet<u64> = data_guard.nodes.iter().copied().collect();
                        for (&idx, &hash) in page.nodes.iter().zip(page.node_hashes.iter()) {
                            if seen.insert(idx) {
                                data_guard.nodes.push(idx);
                                data_guard.node_hashes.push(hash);
                            }
                        }
                    }

                    {
                        let mut avail = lock_recover(
                            available.as_ref(),
                            "streaming_address_queue.available_elements",
                        );
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

        {
            let mut complete_guard =
                lock_recover(complete.as_ref(), "streaming_address_queue.fetch_complete");
            *complete_guard = true;
        }
        ready.notify_all();
        complete_condvar.notify_all();
        tracing::debug!("Background address fetch complete");
    });

    Ok(Some(streaming))
}
