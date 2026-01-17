use std::time::{Duration, SystemTime, UNIX_EPOCH};

use light_client::rpc::{errors::RpcError, Rpc};
use light_registry::{
    protocol_config::state::{ProtocolConfig, ProtocolConfigPda},
    utils::get_protocol_config_pda_address,
};
use tracing::{debug, info, warn};

pub async fn get_protocol_config<R: Rpc>(rpc: &mut R) -> Result<ProtocolConfig, RpcError> {
    let authority_pda = get_protocol_config_pda_address();
    let protocol_config_account = rpc
        .get_anchor_account::<ProtocolConfigPda>(&authority_pda.0)
        .await?
        .ok_or_else(|| RpcError::AccountDoesNotExist(authority_pda.0.to_string()))?;
    debug!("Protocol config account: {:?}", protocol_config_account);
    Ok(protocol_config_account.config)
}

/// Fetches protocol config with infinite retry on rate limit errors.
/// Uses exponential backoff starting at 5 seconds, maxing at 60 seconds.
pub async fn get_protocol_config_with_retry<R: Rpc>(rpc: &mut R) -> ProtocolConfig {
    let mut retry_delay = Duration::from_secs(5);
    let max_delay = Duration::from_secs(60);
    let mut attempt = 0u64;

    loop {
        attempt += 1;
        match get_protocol_config(rpc).await {
            Ok(config) => {
                if attempt > 1 {
                    info!(
                        "Successfully fetched protocol config after {} attempts",
                        attempt
                    );
                }
                return config;
            }
            Err(RpcError::RateLimited) => {
                warn!(
                    "Rate limited fetching protocol config (attempt {}), retrying in {:?}...",
                    attempt, retry_delay
                );
                tokio::time::sleep(retry_delay).await;
                retry_delay = std::cmp::min(retry_delay * 2, max_delay);
            }
            Err(e) => {
                warn!(
                    "Failed to fetch protocol config (attempt {}): {:?}, retrying in {:?}...",
                    attempt, e, retry_delay
                );
                tokio::time::sleep(retry_delay).await;
                retry_delay = std::cmp::min(retry_delay * 2, max_delay);
            }
        }
    }
}

/// Fetches current slot with infinite retry on rate limit errors.
/// Uses exponential backoff starting at 5 seconds, maxing at 60 seconds.
pub async fn get_slot_with_retry<R: Rpc>(rpc: &mut R) -> u64 {
    let mut retry_delay = Duration::from_secs(5);
    let max_delay = Duration::from_secs(60);
    let mut attempt = 0u64;

    loop {
        attempt += 1;
        match rpc.get_slot().await {
            Ok(slot) => {
                if attempt > 1 {
                    info!("Successfully fetched slot after {} attempts", attempt);
                }
                return slot;
            }
            Err(RpcError::RateLimited) => {
                warn!(
                    "Rate limited fetching slot (attempt {}), retrying in {:?}...",
                    attempt, retry_delay
                );
                tokio::time::sleep(retry_delay).await;
                retry_delay = std::cmp::min(retry_delay * 2, max_delay);
            }
            Err(e) => {
                warn!(
                    "Failed to fetch slot (attempt {}): {:?}, retrying in {:?}...",
                    attempt, e, retry_delay
                );
                tokio::time::sleep(retry_delay).await;
                retry_delay = std::cmp::min(retry_delay * 2, max_delay);
            }
        }
    }
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
