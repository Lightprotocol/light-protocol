use std::time::{SystemTime, UNIX_EPOCH};

use light_client::rpc::Rpc;
use light_registry::{
    protocol_config::state::{ProtocolConfig, ProtocolConfigPda},
    utils::get_protocol_config_pda_address,
};
use tracing::{debug, warn};

pub async fn get_protocol_config<R: Rpc>(rpc: &mut R) -> crate::Result<ProtocolConfig> {
    let authority_pda = get_protocol_config_pda_address();
    let protocol_config_account = rpc
        .get_anchor_account::<ProtocolConfigPda>(&authority_pda.0)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch protocol config account: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("Protocol config account not found"))?;
    debug!("Protocol config account: {:?}", protocol_config_account);
    Ok(protocol_config_account.config)
}

pub fn get_current_system_time_ms() -> u128 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_millis(),
        Err(e) => {
            warn!("SystemTime went backwards: {}", e);
            0
        }
    }
}
