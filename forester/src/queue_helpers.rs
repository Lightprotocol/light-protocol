use account_compression::QueueAccount;
use light_batched_merkle_tree::{
    constants::{DEFAULT_ADDRESS_ZKP_BATCH_SIZE, DEFAULT_ZKP_BATCH_SIZE},
    merkle_tree::BatchedMerkleTreeAccount,
    queue::BatchedQueueAccount,
};
use light_client::rpc::Rpc;
use light_hash_set::HashSet;
use solana_sdk::pubkey::Pubkey;
use tracing::trace;

use crate::Result;

#[derive(Debug, Clone)]
pub struct QueueItemData {
    pub hash: [u8; 32],
    pub index: usize,
}

pub async fn fetch_queue_item_data<R: Rpc>(
    rpc: &mut R,
    queue_pubkey: &Pubkey,
    start_index: u16,
    processing_length: u16,
    queue_length: u16,
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
    let queue: HashSet =
        unsafe { HashSet::from_bytes_copy(&mut account.data[8 + size_of::<QueueAccount>()..])? };
    let end_index = (start_index + processing_length).min(queue_length);

    let filtered_queue = queue
        .iter()
        .filter(|(index, cell)| {
            *index >= start_index as usize
                && *index < end_index as usize
                && cell.sequence_number.is_none()
        })
        .map(|(index, cell)| QueueItemData {
            hash: cell.value_bytes(),
            index,
        })
        .collect();
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
