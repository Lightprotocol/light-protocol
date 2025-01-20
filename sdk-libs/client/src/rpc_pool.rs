use std::time::Duration;

use async_trait::async_trait;
use bb8::{Pool, PooledConnection};
use solana_sdk::commitment_config::CommitmentConfig;
use thiserror::Error;
use tokio::time::sleep;
use std::fmt::Debug;
use crate::rpc::{RpcConnection, RpcError};


#[async_trait]
pub trait RpcPool<R: RpcConnection>: Send + Sync + std::fmt::Debug + 'static {
    type Connection<'a>: std::ops::Deref<Target = R> + std::ops::DerefMut + Send + 'a where Self: 'a;
    
    async fn get_connection<'a>(&'a self) -> Result<Self::Connection<'a>, PoolError>;
    async fn get_connection_with_retry<'a>(
        &'a self,
        max_retries: u32,
        delay: Duration,
    ) -> Result<Self::Connection<'a>, PoolError>;
}


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
    _phantom: std::marker::PhantomData<R>,
}

impl<R: RpcConnection> SolanaConnectionManager<R> {
    pub fn new(url: String, commitment: CommitmentConfig) -> Self {
        Self {
            url,
            commitment,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<R: RpcConnection> bb8::ManageConnection for SolanaConnectionManager<R> {
    type Connection = R;
    type Error = PoolError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        Ok(R::new(&self.url, Some(self.commitment)))
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
    ) -> Result<Self, PoolError> {
        let manager = SolanaConnectionManager::new(url, commitment);
        let pool = Pool::builder()
            .max_size(max_size)
            .connection_timeout(Duration::from_secs(15))
            .idle_timeout(Some(Duration::from_secs(60 * 5)))
            .build(manager)
            .await
            .map_err(|e| PoolError::Pool(e.to_string()))?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl<R: RpcConnection> RpcPool<R> for SolanaRpcPool<R> {
    type Connection<'a> = PooledConnection<'a, SolanaConnectionManager<R>>;

    async fn get_connection<'a>(&'a self) -> Result<Self::Connection<'a>, PoolError> {
        self.pool
            .get()
            .await
            .map_err(|e| PoolError::Pool(e.to_string()))
    }

    async fn get_connection_with_retry<'a>(
        &'a self,
        max_retries: u32,
        delay: Duration,
    ) -> Result<Self::Connection<'a>, PoolError> {
        let mut retries = 0;
        loop {
            match self.pool.get().await {
                Ok(conn) => return Ok(conn),
                Err(e) if retries < max_retries => {
                    retries += 1;
                    eprintln!("Failed to get connection (attempt {}): {:?}", retries, e);
                    sleep(delay).await;
                }
                Err(e) => return Err(PoolError::Pool(e.to_string())),
            }
        }
    }
}
