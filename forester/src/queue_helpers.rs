use account_compression::QueueAccount;
use light_batched_merkle_tree::{
    batch::BatchState,
    constants::{DEFAULT_ADDRESS_ZKP_BATCH_SIZE, DEFAULT_ZKP_BATCH_SIZE},
    merkle_tree::BatchedMerkleTreeAccount,
    queue::BatchedQueueAccount,
};
use light_client::rpc::Rpc;
use light_hash_set::HashSet;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use tracing::trace;

use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V2QueueInfo {
    pub next_index: u64,
    pub pending_batch_index: u64,
    pub zkp_batch_size: u64,
    pub batches: Vec<BatchInfo>,
    pub input_pending_batches: u64,
    pub output_pending_batches: u64,
    pub input_items_in_current_zkp_batch: u64,
    pub output_items_in_current_zkp_batch: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchInfo {
    pub batch_index: usize,
    pub state: String,
    pub num_inserted: u64,
    pub current_index: u64,
    pub pending: u64,
    /// Items accumulated in the current partially-filled ZKP batch (0 to zkp_batch_size-1)
    pub items_in_current_zkp_batch: u64,
}

pub fn parse_state_v2_queue_info(
    merkle_tree: &BatchedMerkleTreeAccount,
    output_queue_data: &mut [u8],
) -> crate::Result<V2QueueInfo> {
    let output_queue = BatchedQueueAccount::output_from_bytes(output_queue_data)
        .map_err(|e| anyhow::anyhow!("Failed to parse StateV2 output queue: {:?}", e))?;

    let next_index = output_queue.batch_metadata.next_index;
    let mut zkp_batch_size = DEFAULT_ZKP_BATCH_SIZE;
    let mut output_pending_batches = 0u64;
    let mut batch_infos = Vec::new();

    for (batch_idx, batch) in output_queue.batch_metadata.batches.iter().enumerate() {
        zkp_batch_size = batch.zkp_batch_size;
        let num_inserted = batch.get_num_inserted_zkps();
        let current_index = batch.get_current_zkp_batch_index();
        let pending_in_batch = current_index.saturating_sub(num_inserted);

        batch_infos.push(BatchInfo {
            batch_index: batch_idx,
            state: format!("{:?}", batch.get_state()),
            num_inserted,
            current_index,
            pending: pending_in_batch,
            items_in_current_zkp_batch: batch.get_num_inserted_zkp_batch(),
        });

        output_pending_batches += pending_in_batch;
    }

    let mut input_pending_batches = 0u64;
    let mut input_items_in_current_zkp_batch = 0u64;
    for batch in merkle_tree.queue_batches.batches.iter() {
        let num_inserted = batch.get_num_inserted_zkps();
        let current_index = batch.get_current_zkp_batch_index();
        let pending_in_batch = current_index.saturating_sub(num_inserted);
        input_pending_batches += pending_in_batch;

        if batch.get_state() == BatchState::Fill {
            input_items_in_current_zkp_batch = batch.get_num_inserted_zkp_batch();
        }
    }

    let output_items_in_current_zkp_batch = batch_infos
        .iter()
        .find(|b| b.state == "Fill")
        .map(|b| b.items_in_current_zkp_batch)
        .unwrap_or(0);

    Ok(V2QueueInfo {
        next_index,
        pending_batch_index: output_queue.batch_metadata.pending_batch_index,
        zkp_batch_size,
        batches: batch_infos,
        input_pending_batches,
        output_pending_batches,
        input_items_in_current_zkp_batch,
        output_items_in_current_zkp_batch,
    })
}

pub fn parse_address_v2_queue_info(merkle_tree: &BatchedMerkleTreeAccount) -> V2QueueInfo {
    let next_index = merkle_tree.queue_batches.next_index;
    let mut zkp_batch_size = DEFAULT_ADDRESS_ZKP_BATCH_SIZE;
    let mut pending_batches = 0u64;
    let mut batch_infos = Vec::new();
    let mut input_items_in_current_zkp_batch = 0u64;

    for (batch_idx, batch) in merkle_tree.queue_batches.batches.iter().enumerate() {
        zkp_batch_size = batch.zkp_batch_size;
        let num_inserted = batch.get_num_inserted_zkps();
        let current_index = batch.get_current_zkp_batch_index();
        let pending_in_batch = current_index.saturating_sub(num_inserted);

        if batch.get_state() == BatchState::Fill {
            input_items_in_current_zkp_batch = batch.get_num_inserted_zkp_batch();
        }

        batch_infos.push(BatchInfo {
            batch_index: batch_idx,
            state: format!("{:?}", batch.get_state()),
            num_inserted,
            current_index,
            pending: pending_in_batch,
            items_in_current_zkp_batch: batch.get_num_inserted_zkp_batch(),
        });

        pending_batches += pending_in_batch;
    }

    V2QueueInfo {
        next_index,
        pending_batch_index: merkle_tree.queue_batches.pending_batch_index,
        zkp_batch_size,
        batches: batch_infos,
        input_pending_batches: pending_batches,
        output_pending_batches: 0,
        input_items_in_current_zkp_batch,
        output_items_in_current_zkp_batch: 0,
    }
}

#[derive(Debug, Clone)]
pub struct QueueItemData {
    pub hash: [u8; 32],
    pub index: usize,
}

pub async fn fetch_queue_item_data<R: Rpc>(
    rpc: &mut R,
    queue_pubkey: &Pubkey,
    start_index: u16,
) -> Result<Vec<QueueItemData>> {
    trace!("Fetching queue data for {:?}", queue_pubkey);
    let account = rpc.get_account(*queue_pubkey).await?;
    let mut account = match account {
        Some(acc) => acc,
        None => {
            tracing::warn!(
                "Queue account {} not found - may have been deleted or not yet created",
                queue_pubkey
            );
            return Ok(Vec::new());
        }
    };
    let offset = 8 + std::mem::size_of::<QueueAccount>();
    if account.data.len() < offset {
        tracing::warn!(
            "Queue account {} data too short ({} < {})",
            queue_pubkey,
            account.data.len(),
            offset
        );
        return Ok(Vec::new());
    }
    let queue: HashSet = unsafe { HashSet::from_bytes_copy(&mut account.data[offset..])? };

    let end_index = queue.get_capacity();

    let all_items: Vec<(usize, [u8; 32], bool)> = queue
        .iter()
        .map(|(index, cell)| (index, cell.value_bytes(), cell.sequence_number.is_none()))
        .collect();

    let total_items = all_items.len();
    let total_pending = all_items
        .iter()
        .filter(|(_, _, is_pending)| *is_pending)
        .count();

    let filtered_queue: Vec<QueueItemData> = all_items
        .into_iter()
        .filter(|(index, _, is_pending)| {
            *index >= start_index as usize && *index < end_index && *is_pending
        })
        .map(|(index, hash, _)| QueueItemData { hash, index })
        .collect();

    tracing::info!(
        "Queue {}: total_items={}, total_pending={}, range={}..{}, filtered_result={}",
        queue_pubkey,
        total_items,
        total_pending,
        start_index,
        end_index,
        filtered_queue.len()
    );

    Ok(filtered_queue)
}

pub async fn print_state_v2_output_queue_info<R: Rpc>(
    rpc: &mut R,
    output_queue_pubkey: &Pubkey,
) -> Result<usize> {
    if let Some(mut account) = rpc.get_account(*output_queue_pubkey).await? {
        let output_queue = BatchedQueueAccount::output_from_bytes(account.data.as_mut_slice())?;
        let metadata = output_queue.get_metadata();
        let next_index = metadata.batch_metadata.next_index;

        let mut zkp_batch_size = DEFAULT_ZKP_BATCH_SIZE;
        let mut total_unprocessed = 0;
        let mut batch_details = Vec::new();
        let mut total_completed_operations = 0;

        for (batch_idx, batch) in metadata.batch_metadata.batches.iter().enumerate() {
            zkp_batch_size = batch.zkp_batch_size;
            let num_inserted = batch.get_num_inserted_zkps();
            let current_index = batch.get_current_zkp_batch_index();
            let pending_in_batch = current_index.saturating_sub(num_inserted);

            let completed_operations_in_batch =
                num_inserted * metadata.batch_metadata.zkp_batch_size;
            total_completed_operations += completed_operations_in_batch;

            let pending_operations_in_batch =
                pending_in_batch * metadata.batch_metadata.zkp_batch_size;

            batch_details.push(format!(
                "batch_{}: state={:?}, zkp_inserted={}, zkp_current={}, zkp_pending={}, items_completed={}, items_pending={}",
                batch_idx,
                batch.get_state(),
                num_inserted,
                current_index,
                pending_in_batch,
                completed_operations_in_batch,
                pending_operations_in_batch
            ));

            total_unprocessed += pending_operations_in_batch;
        }

        println!("StateV2 {} APPEND:", output_queue_pubkey);
        println!("  next_index (total ever added): {}", next_index);
        println!(
            "  total_completed_operations: {}",
            total_completed_operations
        );
        println!("  total_unprocessed_items: {}", total_unprocessed);
        println!(
            "  pending_batch_index: {}",
            metadata.batch_metadata.pending_batch_index
        );
        println!(
            "  zkp_batch_size: {}",
            metadata.batch_metadata.zkp_batch_size
        );
        println!(
            "  SUMMARY: {} items added, {} items processed, {} items pending",
            next_index, total_completed_operations, total_unprocessed
        );
        for detail in batch_details {
            println!("  {}", detail);
        }
        println!(
            "  Total pending APPEND operations: {}",
            total_unprocessed / zkp_batch_size
        );

        Ok(total_unprocessed as usize)
    } else {
        Err(anyhow::anyhow!("account not found"))
    }
}

pub async fn print_state_v2_input_queue_info<R: Rpc>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
) -> Result<usize> {
    if let Some(mut account) = rpc.get_account(*merkle_tree_pubkey).await? {
        let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
            account.data.as_mut_slice(),
            &(*merkle_tree_pubkey).into(),
        )?;
        let next_index = merkle_tree.queue_batches.next_index;

        let mut total_unprocessed = 0;
        let mut batch_details = Vec::new();
        let mut total_completed_operations = 0;

        let mut zkp_batch_size = DEFAULT_ZKP_BATCH_SIZE;

        for (batch_idx, batch) in merkle_tree.queue_batches.batches.iter().enumerate() {
            zkp_batch_size = batch.zkp_batch_size;
            let num_inserted = batch.get_num_inserted_zkps();
            let current_index = batch.get_current_zkp_batch_index();
            let pending_in_batch = current_index.saturating_sub(num_inserted);

            let completed_operations_in_batch = num_inserted * batch.zkp_batch_size;
            total_completed_operations += completed_operations_in_batch;

            let pending_operations_in_batch = pending_in_batch * batch.zkp_batch_size;

            batch_details.push(format!(
                "batch_{}: state={:?}, zkp_inserted={}, zkp_current={}, zkp_pending={}, items_completed={}, items_pending={}",
                batch_idx,
                batch.get_state(),
                num_inserted,
                current_index,
                pending_in_batch,
                completed_operations_in_batch,
                pending_operations_in_batch
            ));

            total_unprocessed += pending_operations_in_batch;
        }

        println!("StateV2 {} NULLIFY:", merkle_tree_pubkey);
        println!("  next_index (total ever added): {}", next_index);
        println!(
            "  total_completed_operations: {}",
            total_completed_operations
        );
        println!("  total_unprocessed_items: {}", total_unprocessed);
        println!(
            "  pending_batch_index: {}",
            merkle_tree.queue_batches.pending_batch_index
        );
        println!("  zkp_batch_size: {}", zkp_batch_size);
        println!(
            "  SUMMARY: {} items added, {} items processed, {} items pending",
            next_index, total_completed_operations, total_unprocessed
        );
        for detail in batch_details {
            println!("  {}", detail);
        }
        println!(
            "  Total pending NULLIFY operations: {}",
            total_unprocessed / zkp_batch_size
        );

        Ok(total_unprocessed as usize)
    } else {
        Err(anyhow::anyhow!("account not found"))
    }
}

pub async fn print_address_v2_queue_info<R: Rpc>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
) -> Result<usize> {
    if let Some(mut account) = rpc.get_account(*merkle_tree_pubkey).await? {
        let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
            account.data.as_mut_slice(),
            &(*merkle_tree_pubkey).into(),
        )?;
        let next_index = merkle_tree.queue_batches.next_index;

        let mut zkp_batch_size = DEFAULT_ADDRESS_ZKP_BATCH_SIZE;
        let mut total_unprocessed = 0;
        let mut batch_details = Vec::new();

        for (batch_idx, batch) in merkle_tree.queue_batches.batches.iter().enumerate() {
            zkp_batch_size = batch.zkp_batch_size;
            let num_inserted = batch.get_num_inserted_zkps();
            let current_index = batch.get_current_zkp_batch_index();
            let pending_in_batch = current_index.saturating_sub(num_inserted);

            batch_details.push(format!(
                "batch_{}: state={:?}, inserted={}, current={}, pending={}",
                batch_idx,
                batch.get_state(),
                num_inserted,
                current_index,
                pending_in_batch
            ));

            total_unprocessed += pending_in_batch;
        }

        println!("AddressV2 {}:", merkle_tree_pubkey);
        println!("  next_index (total ever added): {}", next_index);
        println!("  total_unprocessed_items: {}", total_unprocessed);
        println!(
            "  pending_batch_index: {}",
            merkle_tree.queue_batches.pending_batch_index
        );
        println!("  zkp_batch_size: {}", zkp_batch_size);
        for detail in batch_details {
            println!("  {}", detail);
        }

        println!("  Total pending ADDRESS operations: {}", total_unprocessed);

        Ok(total_unprocessed as usize)
    } else {
        Err(anyhow::anyhow!("account not found"))
    }
}

#[derive(Debug)]
pub struct QueueUpdate {
    pub pubkey: Pubkey,
    pub slot: u64,
}

pub async fn get_address_v2_queue_info<R: Rpc>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
) -> Result<V2QueueInfo> {
    if let Some(mut account) = rpc.get_account(*merkle_tree_pubkey).await? {
        let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
            account.data.as_mut_slice(),
            &(*merkle_tree_pubkey).into(),
        )?;
        Ok(parse_address_v2_queue_info(&merkle_tree))
    } else {
        Err(anyhow::anyhow!("account not found"))
    }
}

pub async fn get_state_v2_output_queue_info<R: Rpc>(
    rpc: &mut R,
    queue_pubkey: &Pubkey,
) -> Result<V2QueueInfo> {
    if let Some(mut account) = rpc.get_account(*queue_pubkey).await? {
        let queue = BatchedQueueAccount::output_from_bytes(account.data.as_mut_slice())?;
        let next_index = queue.batch_metadata.next_index;

        let mut zkp_batch_size = DEFAULT_ZKP_BATCH_SIZE;
        let mut total_unprocessed = 0;
        let mut batch_infos = Vec::new();
        let mut output_items_in_current_zkp_batch = 0u64;

        for (batch_idx, batch) in queue.batch_metadata.batches.iter().enumerate() {
            zkp_batch_size = batch.zkp_batch_size;
            let num_inserted = batch.get_num_inserted_zkps();
            let current_index = batch.get_current_zkp_batch_index();
            let pending_in_batch = current_index.saturating_sub(num_inserted);

            if batch.get_state() == BatchState::Fill {
                output_items_in_current_zkp_batch = batch.get_num_inserted_zkp_batch();
            }

            batch_infos.push(BatchInfo {
                batch_index: batch_idx,
                state: format!("{:?}", batch.get_state()),
                num_inserted,
                current_index,
                pending: pending_in_batch,
                items_in_current_zkp_batch: batch.get_num_inserted_zkp_batch(),
            });

            total_unprocessed += pending_in_batch;
        }

        let pending_batches = total_unprocessed;

        Ok(V2QueueInfo {
            next_index,
            pending_batch_index: queue.batch_metadata.pending_batch_index,
            zkp_batch_size,
            batches: batch_infos,
            input_pending_batches: 0,
            output_pending_batches: pending_batches,
            input_items_in_current_zkp_batch: 0,
            output_items_in_current_zkp_batch,
        })
    } else {
        Err(anyhow::anyhow!("account not found"))
    }
}
