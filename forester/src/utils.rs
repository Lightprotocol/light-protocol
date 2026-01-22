use std::{
    future::Future,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use light_client::rpc::{errors::RpcError, Rpc};
use light_registry::{
    protocol_config::state::{ProtocolConfig, ProtocolConfigPda},
    utils::get_protocol_config_pda_address,
};
use tracing::{debug, info, warn};

/// Configuration for retry with exponential backoff.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of attempts. None means infinite retry.
    pub max_attempts: Option<u32>,
    /// Initial delay before first retry.
    pub initial_delay: Duration,
    /// Maximum delay between retries.
    pub max_delay: Duration,
    /// Name of the operation for logging.
    pub operation_name: String,
}

impl RetryConfig {
    /// Creates a new RetryConfig with the given operation name.
    /// Defaults to infinite retry with 5s initial delay and 60s max delay.
    pub fn new(operation_name: impl Into<String>) -> Self {
        Self {
            max_attempts: None, // Infinite by default
            initial_delay: Duration::from_secs(5),
            max_delay: Duration::from_secs(60),
            operation_name: operation_name.into(),
        }
    }

    /// Sets the maximum number of attempts.
    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = Some(max_attempts);
        self
    }

    /// Sets the initial delay.
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Sets the maximum delay.
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self::new("operation")
    }
}

/// Generic retry with exponential backoff.
///
/// Executes the given async operation with retry logic. On failure, waits with
/// exponential backoff before retrying.
///
/// # Arguments
/// * `config` - Retry configuration (max attempts, delays, operation name)
/// * `f` - Async closure that returns `Result<T, E>`. Must be `'static` and own its captures.
///
/// # Returns
/// * `Ok(T)` - On success
/// * `Err(E)` - If max_attempts is reached (only when max_attempts is Some)
///
/// # Example
/// ```ignore
/// let config = RetryConfig::new("bootstrap")
///     .with_max_attempts(3)
///     .with_initial_delay(Duration::from_secs(2));
///
/// let rpc_url = rpc_url.clone();
/// let tracker = tracker.clone();
/// retry_with_backoff(config, move || {
///     let rpc_url = rpc_url.clone();
///     let tracker = tracker.clone();
///     async move {
///         bootstrap_accounts(rpc_url, tracker).await
///     }
/// }).await
/// ```
pub async fn retry_with_backoff<F, Fut, T, E>(config: RetryConfig, mut f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut current_delay = config.initial_delay;
    let mut attempt = 0u32;

    loop {
        attempt += 1;

        match f().await {
            Ok(result) => {
                if attempt > 1 {
                    info!(
                        "{} succeeded after {} attempts",
                        config.operation_name, attempt
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                // Check if we've exhausted max attempts
                if let Some(max) = config.max_attempts {
                    if attempt >= max {
                        warn!(
                            "{} failed after {} attempts: {:?}",
                            config.operation_name, attempt, e
                        );
                        return Err(e);
                    }
                }

                warn!(
                    "{} failed (attempt {}): {:?}, retrying in {:?}...",
                    config.operation_name, attempt, e, current_delay
                );

                tokio::time::sleep(current_delay).await;
                current_delay = std::cmp::min(current_delay * 2, config.max_delay);
            }
        }
    }
}

pub async fn get_protocol_config<R: Rpc>(rpc: &mut R) -> Result<ProtocolConfig, RpcError> {
    let authority_pda = get_protocol_config_pda_address();
    let protocol_config_account = rpc
        .get_anchor_account::<ProtocolConfigPda>(&authority_pda.0)
        .await?
        .ok_or_else(|| RpcError::AccountDoesNotExist(authority_pda.0.to_string()))?;
    debug!("Protocol config account: {:?}", protocol_config_account);
    Ok(protocol_config_account.config)
}

/// Fetches protocol config with infinite retry.
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

/// Fetches current slot with infinite retry.
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
