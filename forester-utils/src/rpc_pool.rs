use std::{
    future::Future, sync::atomic::{AtomicU64, Ordering}, time::{Duration, Instant}
};
use async_trait::async_trait;
use bb8::{Pool, PooledConnection};
use light_client::rpc::{RpcConnection, RpcError};
use solana_sdk::commitment_config::CommitmentConfig;
use thiserror::Error;
use tokio::time::sleep;
use tracing::warn;

use crate::metrics::{
    RPC_POOL_CONNECTIONS,
    RPC_POOL_WAIT_DURATION,
    RPC_POOL_ACQUISITION_TOTAL,
    RPC_POOL_TIMEOUTS,
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
    active_connections: AtomicU64,
    max_size: u32,
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

        Ok(Self {
            pool,
            active_connections: AtomicU64::new(0),
            max_size,
        })
    }

    async fn measure_pool_operation<T>(
        &self, 
        operation_type: &str, 
        f: impl Future<Output = Result<T, PoolError>>
    ) -> Result<T, PoolError> {
        let start = Instant::now();
        let pool_id = self.max_size.to_string();

        let result = f.await;
        
        // Record metrics
        let duration = start.elapsed().as_secs_f64();
        RPC_POOL_WAIT_DURATION
            .with_label_values(&[&pool_id])
            .observe(duration);

        match &result {
            Ok(_) => {
                RPC_POOL_ACQUISITION_TOTAL
                    .with_label_values(&[&pool_id, operation_type])
                    .inc();
            }
            Err(e) => {
                if e.to_string().contains("timeout") {
                    RPC_POOL_TIMEOUTS
                        .with_label_values(&[&pool_id])
                        .inc();
                    RPC_POOL_ACQUISITION_TOTAL
                        .with_label_values(&[&pool_id, "timeout"])
                        .inc();
                } else {
                    RPC_POOL_ACQUISITION_TOTAL
                        .with_label_values(&[&pool_id, "error"])
                        .inc();
                }
            }
        }

        result
    }
}

pub struct TrackedConnection<'a, R: RpcConnection> {
    conn: PooledConnection<'a, SolanaConnectionManager<R>>,
    pool: &'a SolanaRpcPool<R>,
}

impl<R: RpcConnection> std::ops::Deref for TrackedConnection<'_, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}

impl<R: RpcConnection> std::ops::DerefMut for TrackedConnection<'_, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.conn
    }
}

impl<R: RpcConnection> Drop for TrackedConnection<'_, R> {
    fn drop(&mut self) {
        let active = self.pool.active_connections.fetch_sub(1, Ordering::SeqCst) - 1;
        let idle = self.pool.pool.state().idle_connections as i64;
        
        RPC_POOL_CONNECTIONS.with_label_values(&["active"]).set(active as i64);
        RPC_POOL_CONNECTIONS.with_label_values(&["idle"]).set(idle);
    }
}

#[async_trait]
pub trait RpcPool<R: RpcConnection>: Send + Sync + std::fmt::Debug + 'static {
    type Connection<'a>: std::ops::Deref<Target = R> + std::ops::DerefMut + Send + 'a 
    where 
        Self: 'a;
    
    async fn get_connection<'a>(&'a self) -> Result<Self::Connection<'a>, PoolError>;
    
    async fn get_connection_with_retry<'a>(
        &'a self,
        max_retries: u32,
        delay: Duration,
    ) -> Result<Self::Connection<'a>, PoolError>;
}

#[async_trait]
impl<R: RpcConnection> RpcPool<R> for SolanaRpcPool<R> {
    type Connection<'a> = TrackedConnection<'a, R> where Self: 'a;

    async fn get_connection<'a>(&'a self) -> Result<Self::Connection<'a>, PoolError> {
        self.measure_pool_operation("get_connection", async {
            match self.pool.get().await {
                Ok(conn) => {
                    let active = self.active_connections.fetch_add(1, Ordering::SeqCst) + 1;
                    let idle = self.pool.state().idle_connections as i64;
                    
                    RPC_POOL_CONNECTIONS.with_label_values(&["active"]).set(active as i64);
                    RPC_POOL_CONNECTIONS.with_label_values(&["idle"]).set(idle);
                    
                    Ok(TrackedConnection {
                        conn,
                        pool: self,
                    })
                }
                Err(e) => Err(PoolError::Pool(e.to_string())),
            }
        }).await
    }

    async fn get_connection_with_retry<'a>(
        &'a self,
        max_retries: u32,
        delay: Duration,
    ) -> Result<Self::Connection<'a>, PoolError> {
        let mut retries = 0;
        loop {
            match self.get_connection().await {
                Ok(conn) => return Ok(conn),
                Err(e) if retries < max_retries => {
                    retries += 1;
                    warn!("Failed to get connection (attempt {}): {:?}", retries, e);
                    sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}