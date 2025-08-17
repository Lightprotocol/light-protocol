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

/// Constructs a prover URL with the appropriate endpoint path.
///
/// For local validators: uses the base URL as-is (e.g., "localhost/prove")
/// For k8s deployments: appends the operation-specific path (e.g., "url/address-append/prove")
pub fn construct_prover_url(base_url: &str, operation: &str) -> String {
    // Check if this is a local validator URL (contains localhost or 127.0.0.1)
    if base_url.contains("127.0.0.1") || base_url.contains("localhost") {
        // Local validator - use base URL as-is for all operations
        base_url.to_string()
    } else {
        // K8s deployment - append the operation-specific path
        // operation should be like "/address-append", "/update", or "/append"
        format!("{}{}/prove", base_url, operation)
    }
}
