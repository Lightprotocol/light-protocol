use std::time::Duration;

use async_trait::async_trait;
use bb8::{Pool, PooledConnection};
use light_client::rpc::{rpc_connection::RpcConnectionConfig, RpcConnection, RpcError};
use solana_sdk::commitment_config::CommitmentConfig;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{error, trace};

use crate::rate_limiter::RateLimiter;

#[derive(Error, Debug)]
pub enum PoolError {
    #[error("Failed to create RPC client: {0}")]
    ClientCreation(String),
    #[error("RPC request failed: {0}")]
    RpcRequest(#[from] RpcError),
    #[error("Pool error: {0}")]
    Pool(String),
}

pub struct SolanaConnectionManager<R: RpcConnection + 'static> {
    url: String,
    commitment: CommitmentConfig,
    // TODO: implement RpcConnection for SolanaConnectionManager and rate limit requests.
    _rpc_rate_limiter: Option<RateLimiter>,
    _send_tx_rate_limiter: Option<RateLimiter>,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: RpcConnection + 'static> SolanaConnectionManager<R> {
    pub fn new(
        url: String,
        commitment: CommitmentConfig,
        rpc_rate_limiter: Option<RateLimiter>,
        send_tx_rate_limiter: Option<RateLimiter>,
    ) -> Self {
        Self {
            url,
            commitment,
            _rpc_rate_limiter: rpc_rate_limiter,
            _send_tx_rate_limiter: send_tx_rate_limiter,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<R: RpcConnection + 'static> bb8::ManageConnection for SolanaConnectionManager<R> {
    type Connection = R;
    type Error = PoolError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let config = RpcConnectionConfig {
            url: self.url.to_string(),
            commitment_config: Some(self.commitment),
            with_indexer: false,
        };
        Ok(R::new(config))
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        conn.health().await.map_err(PoolError::RpcRequest)
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct SolanaRpcPool<R: RpcConnection + 'static> {
    pool: Pool<SolanaConnectionManager<R>>,
}

impl<R: RpcConnection + 'static> SolanaRpcPool<R> {
    pub async fn new(
        url: String,
        commitment: CommitmentConfig,
        max_size: u32,
        rpc_rate_limiter: Option<RateLimiter>,
        send_tx_rate_limiter: Option<RateLimiter>,
    ) -> Result<Self, PoolError> {
        let manager =
            SolanaConnectionManager::new(url, commitment, rpc_rate_limiter, send_tx_rate_limiter);
        let pool = Pool::builder()
            .max_size(max_size)
            .connection_timeout(Duration::from_secs(15))
            .idle_timeout(Some(Duration::from_secs(60 * 5)))
            .build(manager)
            .await
            .map_err(|e| PoolError::Pool(e.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn get_connection(
        &self,
    ) -> Result<PooledConnection<'_, SolanaConnectionManager<R>>, PoolError> {
        trace!("Attempting to get RPC connection...");
        let result = self
            .pool
            .get()
            .await
            .map_err(|e| PoolError::Pool(e.to_string()));

        match result {
            Ok(_) => {
                trace!("Successfully got RPC connection");
            }
            Err(ref e) => {
                error!("Failed to get RPC connection: {:?}", e);
            }
        }

        result
    }

    pub async fn get_connection_with_retry(
        &self,
        max_retries: u32,
        delay: Duration,
    ) -> Result<PooledConnection<'_, SolanaConnectionManager<R>>, PoolError> {
        let mut retries = 0;
        loop {
            match self.pool.get().await {
                Ok(conn) => return Ok(conn),
                Err(e) if retries < max_retries => {
                    retries += 1;
                    eprintln!("Failed to get connection (attempt {}): {:?}", retries, e);
                    tokio::task::yield_now().await;
                    sleep(delay).await;
                }
                Err(e) => return Err(PoolError::Pool(e.to_string())),
            }
        }
    }
}
