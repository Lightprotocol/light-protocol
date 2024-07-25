use std::sync::Arc;

use tokio::sync::Mutex;

use light_test_utils::rpc::rpc_connection::RpcConnection;

use crate::tree_sync::TreeData;
use crate::{fetch_address_queue_data, fetch_state_queue_data};

pub fn decode_hash(account: &str) -> [u8; 32] {
    let bytes = bs58::decode(account).into_vec().unwrap();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    arr
}

pub fn u8_arr_to_hex_string(arr: &[u8]) -> String {
    arr.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join("")
}

pub async fn get_state_queue_length<R: RpcConnection>(
    rpc: Arc<Mutex<R>>,
    tree_data: TreeData,
) -> usize {
    let queue = fetch_state_queue_data(rpc, tree_data).await.unwrap();
    queue.data.len()
}

pub async fn get_address_queue_length<R: RpcConnection>(
    rpc: Arc<Mutex<R>>,
    tree_data: TreeData,
) -> usize {
    let queue = fetch_address_queue_data(rpc, tree_data).await.unwrap();
    queue.accounts.len()
}
