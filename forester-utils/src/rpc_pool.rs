use std::{cmp::min, time::Duration};

use async_trait::async_trait;
use bb8::{Pool, PooledConnection};
use light_client::rpc::{LightClientConfig, Rpc, RpcError};
use solana_sdk::commitment_config::CommitmentConfig;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{error, trace, warn};

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

pub struct SolanaConnectionManager<R: Rpc + 'static> {
    url: String,
    photon_url: Option<String>,
    api_key: Option<String>,
    commitment: CommitmentConfig,
    // TODO: implement Rpc for SolanaConnectionManager and rate limit requests.
    _rpc_rate_limiter: Option<RateLimiter>,
    _send_tx_rate_limiter: Option<RateLimiter>,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: Rpc + 'static> SolanaConnectionManager<R> {
    pub fn new(
        url: String,
        photon_url: Option<String>,
        api_key: Option<String>,
        commitment: CommitmentConfig,
        rpc_rate_limiter: Option<RateLimiter>,
        send_tx_rate_limiter: Option<RateLimiter>,
    ) -> Self {
        Self {
            url,
            photon_url,
            api_key,
            commitment,
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
        let config = LightClientConfig {
            url: self.url.to_string(),
            photon_url: self.photon_url.clone(),
            commitment_config: Some(self.commitment),
            fetch_active_tree: false,
            api_key: self.api_key.clone(),
        };

        Ok(R::new(config).await?)
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        conn.health().await.map_err(PoolError::RpcRequest)
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct SolanaRpcPool<R: Rpc + 'static> {
    pool: Pool<SolanaConnectionManager<R>>,
    max_retries: u32,
    initial_retry_delay: Duration,
    max_retry_delay: Duration,
}

#[derive(Debug)]
pub struct SolanaRpcPoolBuilder<R: Rpc> {
    url: Option<String>,
    photon_url: Option<String>,
    api_key: Option<String>,
    commitment: Option<CommitmentConfig>,

    max_size: u32,
    connection_timeout_secs: u64,
    idle_timeout_secs: u64,
    max_retries: u32,
    initial_retry_delay_ms: u64,
    max_retry_delay_ms: u64,

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
            api_key: None,
            commitment: None,
            max_size: 50,
            connection_timeout_secs: 15,
            idle_timeout_secs: 300,
            max_retries: 3,
            initial_retry_delay_ms: 1000,
            max_retry_delay_ms: 16000,
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

    pub fn api_key(mut self, api_key: Option<String>) -> Self {
        self.api_key = api_key;
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

    pub async fn build(self) -> Result<SolanaRpcPool<R>, PoolError> {
        let url = self
            .url
            .ok_or_else(|| PoolError::BuilderMissingField("url".to_string()))?;
        let commitment = self
            .commitment
            .ok_or_else(|| PoolError::BuilderMissingField("commitment".to_string()))?;

        let manager = SolanaConnectionManager::new(
            url,
            self.photon_url,
            self.api_key,
            commitment,
            self.rpc_rate_limiter,
            self.send_tx_rate_limiter,
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
        })
    }
}

impl<R: Rpc> SolanaRpcPool<R> {
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
