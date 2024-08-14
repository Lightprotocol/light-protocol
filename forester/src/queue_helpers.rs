use crate::errors::ForesterError;
use account_compression::initialize_address_merkle_tree::Pubkey;
use account_compression::QueueAccount;
use light_hash_set::HashSet;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::debug;
use std::mem;

#[derive(Debug, Clone)]
pub struct QueueItemData {
    pub hash: [u8; 32],
    pub index: usize,
}

pub async fn fetch_queue_item_data<R: RpcConnection>(
    rpc: &mut R,
    queue_pubkey: &Pubkey,
) -> crate::Result<Vec<QueueItemData>> {
    debug!("Fetching queue data for {:?}", queue_pubkey);
    let mut account = rpc
        .get_account(*queue_pubkey)
        .await?
        .ok_or_else(|| ForesterError::Custom("Queue account not found".to_string()))?;

    let nullifier_queue: HashSet = unsafe {
        HashSet::from_bytes_copy(&mut account.data[8 + mem::size_of::<QueueAccount>()..])?
    };

    Ok(nullifier_queue
        .iter()
        .filter_map(|(index, cell)| {
            if cell.sequence_number.is_none() {
                Some(QueueItemData {
                    hash: cell.value_bytes(),
                    index,
                })
            } else {
                None
            }
        })
        .collect())
}

#[derive(Debug)]
pub struct QueueUpdate {
    pub(crate) pubkey: Pubkey,
    pub(crate) slot: u64,
}
