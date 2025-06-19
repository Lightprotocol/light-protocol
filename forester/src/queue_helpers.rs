use account_compression::QueueAccount;
use light_batched_merkle_tree::{
    batch::BatchState,
    merkle_tree::{self, BatchedMerkleTreeAccount},
    queue::BatchedQueueAccount,
};
use light_client::rpc::Rpc;
use light_hash_set::HashSet;
use solana_sdk::pubkey::Pubkey;
use tracing::{debug, instrument, trace};

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
    merkle_tree_pubkey: &Pubkey,
) -> Result<usize> {
    trace!(
        "Fetching StateV2 queue length for {:?}",
        output_queue_pubkey
    );

    debug!(
        "Fetching StateV2 queue length for queue {:?} tree {:?} ({:?})",
        output_queue_pubkey,
        merkle_tree_pubkey,
        merkle_tree_pubkey.to_bytes()
    );

    if let Some(mut account) = rpc.get_account(*output_queue_pubkey).await? {
        let output_queue = BatchedQueueAccount::output_from_bytes(account.data.as_mut_slice())?;

        let batch_metadata = &output_queue.get_metadata().batch_metadata;
        let zkp_batch_size = batch_metadata.zkp_batch_size as usize;

        let mut queue_length: usize = 0;
        for batch in batch_metadata.batches {
            if batch.get_state() == BatchState::Inserted {
                continue;
            }
            let total_zkp_batches = batch.get_num_zkp_batches() as usize;
            let inserted_zkp_batches = batch.get_num_inserted_zkps() as usize;
            let remaining_zkp_batches = total_zkp_batches.saturating_sub(inserted_zkp_batches);

            queue_length += remaining_zkp_batches * zkp_batch_size;
        }

        Ok(queue_length)
    } else {
        Err(anyhow::anyhow!("account not found"))
    }
}

#[instrument(level = "debug", skip(rpc))]
pub async fn fetch_address_v2_queue_length<R: Rpc>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
) -> Result<usize> {
    debug!(
        "Fetching AddressV2 queue length for {:?} ({:?})",
        merkle_tree_pubkey,
        merkle_tree_pubkey.to_bytes()
    );

    if let Some(mut account) = rpc.get_account(*merkle_tree_pubkey).await? {
        let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
            account.data.as_mut_slice(),
            &(*merkle_tree_pubkey).into(),
        )?;

        let mut queue_length: usize = 0;
        let mut batch_index = 0;
        for batch in merkle_tree.queue_batches.batches {
            if batch.get_state() == BatchState::Inserted {
                continue;
            }
            let zkp_batch_size = batch.zkp_batch_size as usize;

            let total_zkp_batches = batch.get_num_zkp_batches() as usize;
            let inserted_zkp_batches = batch.get_num_inserted_zkps() as usize;
            let remaining_zkp_batches = total_zkp_batches.saturating_sub(inserted_zkp_batches);

            debug!("batch {} state: {:?}", batch_index, batch.get_state());
            debug!("batch {} zkp_batch_size: {}", batch_index, zkp_batch_size);
            debug!(
                "batch {} total_zkp_batches: {}",
                batch_index, total_zkp_batches
            );
            debug!(
                "batch {} get_current_zkp_batch_index: {}",
                batch_index,
                batch.get_current_zkp_batch_index()
            );
            debug!(
                "batch {} inserted_zkp_batches: {}",
                batch_index, inserted_zkp_batches
            );
            debug!(
                "batch {} remaining_zkp_batches: {}",
                batch_index, remaining_zkp_batches
            );
            debug!(
                "batch {} is ready to insert? {:?}",
                batch_index,
                batch.batch_is_ready_to_insert()
            );

            queue_length += remaining_zkp_batches * zkp_batch_size;
            batch_index += 1;
        }

        Ok(queue_length)
    } else {
        Err(anyhow::anyhow!("account not found"))
    }
}

#[derive(Debug)]
pub struct QueueUpdate {
    pub pubkey: Pubkey,
    pub slot: u64,
}
