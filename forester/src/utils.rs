use std::time::{SystemTime, UNIX_EPOCH};

use light_client::rpc::RpcConnection;
use light_registry::{
    protocol_config::state::{ProtocolConfig, ProtocolConfigPda},
    utils::get_protocol_config_pda_address,
};
use tracing::debug;

pub fn decode_hash(account: &str) -> [u8; 32] {
    let bytes = bs58::decode(account).into_vec().unwrap();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    arr
}

pub async fn get_protocol_config<R: RpcConnection>(rpc: &mut R) -> ProtocolConfig {
    let authority_pda = get_protocol_config_pda_address();
    let protocol_config_account = rpc
        .get_anchor_account::<ProtocolConfigPda>(&authority_pda.0)
        .await
        .unwrap()
        .unwrap();
    debug!("Protocol config account: {:?}", protocol_config_account);
    protocol_config_account.config
}

pub fn get_current_system_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
}
