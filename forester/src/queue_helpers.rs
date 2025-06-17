use account_compression::QueueAccount;
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
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
    let mut account = rpc.get_account(*queue_pubkey).await?.unwrap();
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

pub async fn fetch_state_v2_queue_length<R: Rpc>(
    rpc: &mut R,
    output_queue_pubkey: &Pubkey,
) -> Result<usize> {
    trace!(
        "Fetching StateV2 queue length for {:?}",
        output_queue_pubkey
    );
    if let Some(mut account) = rpc.get_account(*output_queue_pubkey).await? {
        let output_queue = BatchedQueueAccount::output_from_bytes(account.data.as_mut_slice())?;

        let batch_metadata = &output_queue.get_metadata().batch_metadata;
        let pending_batch_index = batch_metadata.pending_batch_index as usize;

        // Get the pending batch
        if let Some(pending_batch) = batch_metadata.batches.get(pending_batch_index) {
            // Calculate items ready for processing in the current pending batch
            let zkp_batch_size = batch_metadata.zkp_batch_size as usize;
            let total_zkp_batches = pending_batch.get_num_zkp_batches() as usize;
            let inserted_zkp_batches = pending_batch.get_num_inserted_zkps() as usize;
            let remaining_zkp_batches = total_zkp_batches.saturating_sub(inserted_zkp_batches);

            Ok(remaining_zkp_batches * zkp_batch_size)
        } else {
            Ok(0)
        }
    } else {
        Err(anyhow::anyhow!("account not found"))
    }
}

pub async fn fetch_address_v2_queue_length<R: Rpc>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
) -> Result<usize> {
    trace!(
        "Fetching AddressV2 queue length for {:?}",
        merkle_tree_pubkey
    );
    if let Some(mut account) = rpc.get_account(*merkle_tree_pubkey).await? {
        let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
            account.data.as_mut_slice(),
            &(*merkle_tree_pubkey).into(),
        )?;

        let pending_batch_index = merkle_tree.queue_batches.pending_batch_index as usize;

        // Get the pending batch
        if let Some(pending_batch) = merkle_tree.queue_batches.batches.get(pending_batch_index) {
            // Calculate items ready for processing in the current pending batch
            let zkp_batch_size = merkle_tree.queue_batches.zkp_batch_size as usize;
            let total_zkp_batches = pending_batch.get_num_zkp_batches() as usize;
            let inserted_zkp_batches = pending_batch.get_num_inserted_zkps() as usize;
            let remaining_zkp_batches = total_zkp_batches.saturating_sub(inserted_zkp_batches);

            Ok(remaining_zkp_batches * zkp_batch_size)
        } else {
            Ok(0)
        }
    } else {
        Err(anyhow::anyhow!("account not found"))
    }
}

#[derive(Debug)]
pub struct QueueUpdate {
    pub pubkey: Pubkey,
    pub slot: u64,
}
