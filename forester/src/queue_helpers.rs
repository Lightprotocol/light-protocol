use crate::{errors::ForesterError, Result};
use account_compression::initialize_address_merkle_tree::Pubkey;
use account_compression::QueueAccount;
use forester_utils::rpc::RpcConnection;
use light_hash_set::HashSet;
use std::mem;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct QueueItemData {
    pub hash: [u8; 32],
    pub index: usize,
}

pub async fn fetch_queue_item_data<R: RpcConnection>(
    rpc: &mut R,
    queue_pubkey: &Pubkey,
    start_index: u16,
    length: u16,
    queue_length: u16,
) -> Result<Vec<QueueItemData>> {
    debug!("Fetching queue data for {:?}", queue_pubkey);
    let mut account = rpc
        .get_account(*queue_pubkey)
        .await?
        .ok_or_else(|| ForesterError::Custom("Queue account not found".to_string()))?;
    let queue: HashSet = unsafe {
        HashSet::from_bytes_copy(&mut account.data[8 + mem::size_of::<QueueAccount>()..])?
    };
    let end_index = (start_index + length).min(queue_length);

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
    debug!("Queue data fetched: {:?}", filtered_queue);
    Ok(filtered_queue)
}

#[derive(Debug)]
pub struct QueueUpdate {
    pub pubkey: Pubkey,
    pub slot: u64,
}
