use crate::ForesterConfig;
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct PipelineContext<T: Indexer, R: RpcConnection> {
    pub indexer: Arc<Mutex<T>>,
    pub rpc: Arc<Mutex<R>>,
    pub config: Arc<ForesterConfig>,
    pub successful_nullifications: Arc<Mutex<usize>>,
}

impl<T: Indexer, R: RpcConnection> Clone for PipelineContext<T, R> {
    fn clone(&self) -> Self {
        PipelineContext {
            indexer: Arc::clone(&self.indexer),
            rpc: Arc::clone(&self.rpc),
            config: Arc::clone(&self.config),
            successful_nullifications: Arc::clone(&self.successful_nullifications),
        }
    }
}
