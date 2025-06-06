use account_compression::QueueAccount;
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

#[derive(Debug)]
pub struct QueueUpdate {
    pub pubkey: Pubkey,
    pub slot: u64,
}
