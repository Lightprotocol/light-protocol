use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use bb8::{Pool, PooledConnection};
use async_trait::async_trait;
use solana_sdk::commitment_config::CommitmentConfig;
use tokio::time::sleep;

use crate::metrics::{
    RPC_POOL_CONNECTIONS,
    RPC_POOL_WAIT_DURATION,
    RPC_POOL_ACQUISITION_TOTAL,
    RPC_POOL_TIMEOUTS,
};
use light_client::rpc::RpcConnection;
use light_client::rpc_pool::{PoolError, RpcPool};

use super::metrics_rpc_connection::MetricsRpcConnection;

pub struct MetricsConnectionManager<R: RpcConnection> {
    url: String,
    commitment: CommitmentConfig,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: RpcConnection> MetricsConnectionManager<R> {
    pub fn new(url: String, commitment: CommitmentConfig) -> Self {
        Self {
            url,
            commitment,
            _phantom: std::marker::PhantomData,
        }
    }
}


#[async_trait]
impl<R: RpcConnection> bb8::ManageConnection for MetricsConnectionManager<R> {
    type Connection = MetricsRpcConnection<R>;
    type Error = PoolError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        Ok(MetricsRpcConnection::new(&self.url, Some(self.commitment)))
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        conn.health().await.map_err(PoolError::RpcRequest)
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct MetricsRpcPool<R: RpcConnection> {
    inner: Pool<MetricsConnectionManager<R>>,
    active_connections: AtomicU64,
    max_size: u32,
}

impl<R: RpcConnection> MetricsRpcPool<R> {
    pub async fn new(
        url: String,
        commitment: CommitmentConfig,
        max_size: u32,
    ) -> Result<Self, PoolError> {
        let manager = MetricsConnectionManager::new(url, commitment);
        let pool = Pool::builder()
            .max_size(max_size)
            .build(manager)
            .await
            .map_err(|e| PoolError::Pool(e.to_string()))?;

        Ok(Self {
            inner: pool,
            active_connections: AtomicU64::new(0),
            max_size,
        })
    }
}

pub struct MetricsPooledConnection<'a, R: RpcConnection> {
    conn: PooledConnection<'a, MetricsConnectionManager<R>>,
    pool: &'a MetricsRpcPool<R>,
}

impl<'a, R: RpcConnection> std::ops::Deref for MetricsPooledConnection<'a, R> {
    type Target = MetricsRpcConnection<R>;

    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}

impl<'a, R: RpcConnection> std::ops::DerefMut for MetricsPooledConnection<'a, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.conn
    }
}

impl<'a, R: RpcConnection> Drop for MetricsPooledConnection<'a, R> {
    fn drop(&mut self) {
        let active = self.pool.active_connections.fetch_sub(1, Ordering::SeqCst) - 1;
        let idle = self.pool.inner.state().idle_connections as i64;
        
        RPC_POOL_CONNECTIONS.with_label_values(&["active"]).set(active as i64);
        RPC_POOL_CONNECTIONS.with_label_values(&["idle"]).set(idle);
    }
}

#[async_trait]
impl<R: RpcConnection> RpcPool<MetricsRpcConnection<R>> for MetricsRpcPool<R> {
    type Connection<'a> = MetricsPooledConnection<'a, R> where Self: 'a;

    async fn get_connection<'a>(&'a self) -> Result<Self::Connection<'a>, PoolError> {
        let start = std::time::Instant::now();
        let pool_id = self.max_size.to_string();
        
        let result = self.inner.get().await;
        
        let duration = start.elapsed().as_secs_f64();
        RPC_POOL_WAIT_DURATION
            .with_label_values(&[&pool_id])
            .observe(duration);

        match result {
            Ok(conn) => {
                RPC_POOL_ACQUISITION_TOTAL
                    .with_label_values(&[&pool_id, "success"])
                    .inc();
                
                let active = self.active_connections.fetch_add(1, Ordering::SeqCst) + 1;
                let idle = self.inner.state().idle_connections as i64;
                
                RPC_POOL_CONNECTIONS.with_label_values(&["active"]).set(active as i64);
                RPC_POOL_CONNECTIONS.with_label_values(&["idle"]).set(idle);
                
                Ok(MetricsPooledConnection {
                    conn,
                    pool: self,
                })
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
                Err(PoolError::Pool(e.to_string()))
            }
        }
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
                    eprintln!("Failed to get connection (attempt {}): {:?}", retries, e);
                    sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}