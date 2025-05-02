use std::time::Duration;

use async_trait::async_trait;
use bb8::{Pool, PooledConnection};
use solana_sdk::commitment_config::CommitmentConfig;
use thiserror::Error;
use tokio::time::sleep;

use crate::{
    rate_limiter::RateLimiter,
    rpc::{RpcConnection, RpcError},
};

#[derive(Error, Debug)]
pub enum PoolError {
    #[error("Failed to create RPC client: {0}")]
    ClientCreation(String),
    #[error("RPC request failed: {0}")]
    RpcRequest(#[from] RpcError),
    #[error("Pool error: {0}")]
    Pool(String),
}

pub struct SolanaConnectionManager<R: RpcConnection> {
    url: String,
    commitment: CommitmentConfig,
    rpc_rate_limiter: Option<RateLimiter>,
    send_tx_rate_limiter: Option<RateLimiter>,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: RpcConnection> SolanaConnectionManager<R> {
    pub fn new(
        url: String,
        commitment: CommitmentConfig,
        rpc_rate_limiter: Option<RateLimiter>,
        send_tx_rate_limiter: Option<RateLimiter>,
    ) -> Self {
        Self {
            url,
            commitment,
            rpc_rate_limiter,
            send_tx_rate_limiter,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<R: RpcConnection> bb8::ManageConnection for SolanaConnectionManager<R> {
    type Connection = R;
    type Error = PoolError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let mut conn = R::new(&self.url, Some(self.commitment));
        if let Some(limiter) = &self.rpc_rate_limiter {
            conn.set_rpc_rate_limiter(limiter.clone());
        }
        if let Some(limiter) = &self.send_tx_rate_limiter {
            conn.set_send_tx_rate_limiter(limiter.clone());
        }
        Ok(conn)
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        conn.health().await.map_err(PoolError::RpcRequest)
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct SolanaRpcPool<R: RpcConnection> {
    pool: Pool<SolanaConnectionManager<R>>,
}

impl<R: RpcConnection> SolanaRpcPool<R> {
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
        self.pool
            .get()
            .await
            .map_err(|e| PoolError::Pool(e.to_string()))
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
