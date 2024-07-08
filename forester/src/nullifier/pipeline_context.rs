use crate::{ForesterConfig, RpcPool};
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct PipelineContext<T: Indexer<R>, R: RpcConnection> {
    pub indexer: Arc<Mutex<T>>,
    pub rpc_pool: RpcPool<R>,
    pub config: Arc<ForesterConfig>,
    pub successful_nullifications: Arc<Mutex<usize>>,
}

impl<T: Indexer<R>, R: RpcConnection> Clone for PipelineContext<T, R> {
    fn clone(&self) -> Self {
        PipelineContext {
            indexer: Arc::clone(&self.indexer),
            rpc_pool: self.rpc_pool.clone(),
            config: Arc::clone(&self.config),
            successful_nullifications: Arc::clone(&self.successful_nullifications),
        }
    }
}
