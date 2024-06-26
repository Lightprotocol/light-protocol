use crate::config::ForesterConfig;
use crate::v2::state::get_nullifier_queue;
use light_test_utils::rpc::rpc_connection::RpcConnection;

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
    rpc: &mut R,
    config: &ForesterConfig,
) -> usize {
    let queue = get_nullifier_queue(&config.nullifier_queue_pubkey, rpc)
        .await
        .unwrap();
    queue.len()
}
