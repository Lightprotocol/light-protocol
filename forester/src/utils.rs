use std::time::{SystemTime, UNIX_EPOCH};

use light_client::rpc::Rpc;
use light_registry::{
    protocol_config::state::{ProtocolConfig, ProtocolConfigPda},
    utils::get_protocol_config_pda_address,
};
use tracing::debug;

pub async fn get_protocol_config<R: Rpc>(rpc: &mut R) -> ProtocolConfig {
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
