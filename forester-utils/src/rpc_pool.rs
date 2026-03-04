use std::{
    cmp::min,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use async_trait::async_trait;
use bb8::{Pool, PooledConnection};
use light_client::rpc::{LightClientConfig, Rpc, RpcError};
use solana_sdk::commitment_config::CommitmentConfig;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{error, info, trace, warn};

use crate::rate_limiter::RateLimiter;

#[derive(Error, Debug)]
pub enum PoolError {
    #[error("Failed to create RPC client: {0}")]
    ClientCreation(String),
    #[error("RPC request failed: {0}")]
    RpcRequest(#[from] RpcError),
    #[error("Pool error: {0}")]
    Pool(String),
    #[error("Failed to get connection after {0} retries: {1}")]
    MaxRetriesExceeded(u32, String),
    #[error("Missing required field for RpcPoolBuilder: {0}")]
    BuilderMissingField(String),
}

/// Shared health state for tracking primary RPC health across all pooled connections.
/// When consecutive `is_valid()` failures cross the threshold, the pool flips to
/// "fallback mode" — new connections try the fallback URL first, and stale primary
/// connections are eagerly dropped by `has_broken()`.
pub struct HealthState {
    use_fallback: AtomicBool,
    consecutive_failures: AtomicU64,
    failure_threshold: u64,
    primary_url: String,
}

impl HealthState {
    pub fn new(failure_threshold: u64, primary_url: String) -> Self {
        Self {
            use_fallback: AtomicBool::new(false),
            consecutive_failures: AtomicU64::new(0),
            failure_threshold,
            primary_url,
        }
    }

    pub fn record_failure(&self) {
        let prev = self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
        if prev + 1 >= self.failure_threshold && !self.use_fallback.swap(true, Ordering::Release) {
            warn!(
                "Primary RPC health check failed {} consecutive times, switching to fallback mode",
                prev + 1
            );
        }
    }

    pub fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::Relaxed);
    }

    pub fn is_fallback_active(&self) -> bool {
        self.use_fallback.load(Ordering::Acquire)
    }

    pub fn recover_primary(&self) {
        if self.use_fallback.swap(false, Ordering::Release) {
            self.consecutive_failures.store(0, Ordering::Relaxed);
            info!("Primary RPC recovered, switching back from fallback mode");
        }
    }
}

impl std::fmt::Debug for HealthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HealthState")
            .field("use_fallback", &self.use_fallback.load(Ordering::Relaxed))
            .field(
                "consecutive_failures",
                &self.consecutive_failures.load(Ordering::Relaxed),
            )
            .field("failure_threshold", &self.failure_threshold)
            .finish()
    }
}

pub struct SolanaConnectionManager<R: Rpc + 'static> {
    url: String,
    photon_url: Option<String>,
    fallback_rpc_url: Option<String>,
    fallback_photon_url: Option<String>,
    commitment: CommitmentConfig,
    health_state: Arc<HealthState>,
    has_fallback: bool,
    // TODO: implement Rpc for SolanaConnectionManager and rate limit requests.
    _rpc_rate_limiter: Option<RateLimiter>,
    _send_tx_rate_limiter: Option<RateLimiter>,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: Rpc + 'static> SolanaConnectionManager<R> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        url: String,
        photon_url: Option<String>,
        fallback_rpc_url: Option<String>,
        fallback_photon_url: Option<String>,
        commitment: CommitmentConfig,
        rpc_rate_limiter: Option<RateLimiter>,
        send_tx_rate_limiter: Option<RateLimiter>,
        health_state: Arc<HealthState>,
    ) -> Self {
        let has_fallback = fallback_rpc_url.is_some();
        Self {
            url,
            photon_url,
            fallback_rpc_url,
            fallback_photon_url,
            commitment,
            health_state,
            has_fallback,
            _rpc_rate_limiter: rpc_rate_limiter,
            _send_tx_rate_limiter: send_tx_rate_limiter,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<R: Rpc + 'static> bb8::ManageConnection for SolanaConnectionManager<R> {
    type Connection = R;
    type Error = PoolError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        // When in fallback mode, try fallback URL first; otherwise try primary first.
        let (first_url, first_photon, second_url, second_photon) =
            if self.health_state.is_fallback_active() {
                (
                    self.fallback_rpc_url.as_deref().unwrap_or(&self.url),
                    self.fallback_photon_url
                        .clone()
                        .or_else(|| self.photon_url.clone()),
                    Some(self.url.as_str()),
                    self.photon_url.clone(),
                )
            } else {
                (
                    self.url.as_str(),
                    self.photon_url.clone(),
                    self.fallback_rpc_url.as_deref(),
                    self.fallback_photon_url
                        .clone()
                        .or_else(|| self.photon_url.clone()),
                )
            };

        let config = LightClientConfig {
            url: first_url.to_string(),
            photon_url: first_photon,
            commitment_config: Some(self.commitment),
            fetch_active_tree: false,
        };

        match R::new(config).await {
            Ok(conn) => Ok(conn),
            Err(first_err) => {
                if let Some(second) = second_url {
                    warn!(
                        "RPC {} failed ({}), trying alternate: {}",
                        first_url, first_err, second
                    );
                    let fallback_config = LightClientConfig {
                        url: second.to_string(),
                        photon_url: second_photon,
                        commitment_config: Some(self.commitment),
                        fetch_active_tree: false,
                    };
                    R::new(fallback_config).await.map_err(|second_err| {
                        error!(
                            "Both RPC endpoints failed: first={}, second={}",
                            first_err, second_err
                        );
                        PoolError::ClientCreation(format!(
                            "first: {}, second: {}",
                            first_err, second_err
                        ))
                    })
                } else {
                    Err(PoolError::ClientCreation(first_err.to_string()))
                }
            }
        }
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        conn.health().await.map_err(|e| {
            // Only track failures for fallback switching when a fallback URL exists.
            if self.has_fallback {
                self.health_state.record_failure();
            }
            PoolError::RpcRequest(e)
        })?;
        // Reset consecutive failure count on success. Note: this does NOT reset
        // use_fallback — that is handled by the recovery probe to avoid flapping.
        if self.has_fallback {
            self.health_state.record_success();
        }
        Ok(())
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        if !self.has_fallback || !self.health_state.is_fallback_active() {
            return false;
        }
        // In fallback mode: connections still pointing to the primary URL are stale.
        // Tell bb8 to drop them so new connections go through connect() → fallback.
        conn.get_url() == self.health_state.primary_url
    }
}

#[derive(Debug)]
pub struct SolanaRpcPool<R: Rpc + 'static> {
    pool: Pool<SolanaConnectionManager<R>>,
    max_retries: u32,
    initial_retry_delay: Duration,
    max_retry_delay: Duration,
    health_state: Arc<HealthState>,
    has_fallback: bool,
    primary_url: String,
    primary_photon_url: Option<String>,
    commitment: CommitmentConfig,
    primary_probe_interval: Duration,
}

#[derive(Debug)]
pub struct SolanaRpcPoolBuilder<R: Rpc> {
    url: Option<String>,
    photon_url: Option<String>,
    fallback_rpc_url: Option<String>,
    fallback_photon_url: Option<String>,
    commitment: Option<CommitmentConfig>,

    max_size: u32,
    connection_timeout_secs: u64,
    idle_timeout_secs: u64,
    max_retries: u32,
    initial_retry_delay_ms: u64,
    max_retry_delay_ms: u64,
    failure_threshold: u64,
    primary_probe_interval_secs: u64,

    rpc_rate_limiter: Option<RateLimiter>,
    send_tx_rate_limiter: Option<RateLimiter>,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: Rpc> Default for SolanaRpcPoolBuilder<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: Rpc> SolanaRpcPoolBuilder<R> {
    pub fn new() -> Self {
        Self {
            url: None,
            photon_url: None,
            fallback_rpc_url: None,
            fallback_photon_url: None,
            commitment: None,
            max_size: 50,
            connection_timeout_secs: 15,
            idle_timeout_secs: 300,
            max_retries: 3,
            initial_retry_delay_ms: 1000,
            max_retry_delay_ms: 16000,
            failure_threshold: 3,
            primary_probe_interval_secs: 30,
            rpc_rate_limiter: None,
            send_tx_rate_limiter: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    pub fn photon_url(mut self, url: Option<String>) -> Self {
        self.photon_url = url;
        self
    }

    pub fn fallback_rpc_url(mut self, url: Option<String>) -> Self {
        self.fallback_rpc_url = url;
        self
    }

    pub fn fallback_photon_url(mut self, url: Option<String>) -> Self {
        self.fallback_photon_url = url;
        self
    }

    pub fn commitment(mut self, commitment: CommitmentConfig) -> Self {
        self.commitment = Some(commitment);
        self
    }

    pub fn max_size(mut self, max_size: u32) -> Self {
        self.max_size = max_size;
        self
    }

    pub fn connection_timeout_secs(mut self, secs: u64) -> Self {
        self.connection_timeout_secs = secs;
        self
    }

    pub fn idle_timeout_secs(mut self, secs: u64) -> Self {
        self.idle_timeout_secs = secs;
        self
    }

    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub fn initial_retry_delay_ms(mut self, ms: u64) -> Self {
        self.initial_retry_delay_ms = ms;
        self
    }

    pub fn max_retry_delay_ms(mut self, ms: u64) -> Self {
        self.max_retry_delay_ms = ms;
        self
    }

    pub fn rpc_rate_limiter(mut self, limiter: RateLimiter) -> Self {
        self.rpc_rate_limiter = Some(limiter);
        self
    }

    pub fn send_tx_rate_limiter(mut self, limiter: RateLimiter) -> Self {
        self.send_tx_rate_limiter = Some(limiter);
        self
    }

    pub fn failure_threshold(mut self, threshold: u64) -> Self {
        self.failure_threshold = threshold;
        self
    }

    pub fn primary_probe_interval_secs(mut self, secs: u64) -> Self {
        self.primary_probe_interval_secs = secs;
        self
    }

    pub async fn build(self) -> Result<SolanaRpcPool<R>, PoolError> {
        let url = self
            .url
            .ok_or_else(|| PoolError::BuilderMissingField("url".to_string()))?;
        let commitment = self
            .commitment
            .ok_or_else(|| PoolError::BuilderMissingField("commitment".to_string()))?;

        let has_fallback = self.fallback_rpc_url.is_some();
        let health_state = Arc::new(HealthState::new(self.failure_threshold, url.clone()));

        let manager = SolanaConnectionManager::new(
            url.clone(),
            self.photon_url.clone(),
            self.fallback_rpc_url,
            self.fallback_photon_url,
            commitment,
            self.rpc_rate_limiter,
            self.send_tx_rate_limiter,
            Arc::clone(&health_state),
        );

        let pool = Pool::builder()
            .max_size(self.max_size)
            .connection_timeout(Duration::from_secs(self.connection_timeout_secs))
            .idle_timeout(Some(Duration::from_secs(self.idle_timeout_secs)))
            .build(manager)
            .await
            .map_err(|e| PoolError::Pool(e.to_string()))?;

        Ok(SolanaRpcPool {
            pool,
            max_retries: self.max_retries,
            initial_retry_delay: Duration::from_millis(self.initial_retry_delay_ms),
            max_retry_delay: Duration::from_millis(self.max_retry_delay_ms),
            health_state,
            has_fallback,
            primary_url: url,
            primary_photon_url: self.photon_url,
            commitment,
            primary_probe_interval: Duration::from_secs(self.primary_probe_interval_secs),
        })
    }
}

impl<R: Rpc> SolanaRpcPool<R> {
    /// Spawns a background task that periodically probes the primary RPC URL
    /// when in fallback mode. When the primary becomes healthy again, switches
    /// back automatically. Returns None if no fallback URL is configured
    /// (fallback mode can never activate).
    pub fn spawn_primary_recovery_probe(self: &Arc<Self>) -> Option<tokio::task::JoinHandle<()>> {
        // Only meaningful if a fallback URL is configured; without one, the
        // health state never flips to fallback mode so there is nothing to recover from.
        if !self.has_fallback {
            return None;
        }
        let health_state = Arc::clone(&self.health_state);

        let primary_url = self.primary_url.clone();
        let primary_photon_url = self.primary_photon_url.clone();
        let commitment = self.commitment;
        let interval = self.primary_probe_interval;

        Some(tokio::spawn(async move {
            loop {
                sleep(interval).await;
                if !health_state.is_fallback_active() {
                    continue;
                }
                // Try connecting to the primary URL.
                let config = LightClientConfig {
                    url: primary_url.clone(),
                    photon_url: primary_photon_url.clone(),
                    commitment_config: Some(commitment),
                    fetch_active_tree: false,
                };
                match R::new(config).await {
                    Ok(conn) => match conn.health().await {
                        Ok(()) => {
                            health_state.recover_primary();
                        }
                        Err(e) => {
                            trace!("Primary RPC probe health check failed: {}", e);
                        }
                    },
                    Err(e) => {
                        trace!("Primary RPC probe connection failed: {}", e);
                    }
                }
            }
        }))
    }

    pub async fn get_connection(
        &self,
    ) -> Result<PooledConnection<'_, SolanaConnectionManager<R>>, PoolError> {
        let mut current_retries = 0;
        let mut current_delay = self.initial_retry_delay;

        loop {
            trace!(
                "Attempting to get RPC connection... (Attempt {})",
                current_retries + 1
            );
            match self.pool.get().await {
                Ok(conn) => {
                    trace!(
                        "Successfully got RPC connection (Attempt {})",
                        current_retries + 1
                    );
                    return Ok(conn);
                }
                Err(e) => {
                    error!(
                        "Failed to get RPC connection (Attempt {}): {:?}",
                        current_retries + 1,
                        e
                    );
                    if current_retries < self.max_retries {
                        current_retries += 1;
                        warn!(
                            "Retrying to get RPC connection in {:?} (Attempt {}/{})",
                            current_delay,
                            current_retries + 1,
                            self.max_retries + 1
                        );
                        tokio::task::yield_now().await;
                        sleep(current_delay).await;
                        current_delay = min(current_delay * 2, self.max_retry_delay);
                    } else {
                        error!(
                            "Failed to get RPC connection after {} attempts. Last error: {:?}",
                            self.max_retries + 1,
                            e
                        );
                        return Err(PoolError::MaxRetriesExceeded(
                            self.max_retries + 1,
                            e.to_string(),
                        ));
                    }
                }
            }
        }
    }
}
