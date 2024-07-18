use std::sync::Arc;

use tokio::sync::Mutex;

use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;

use crate::rollover::RolloverState;
use crate::tree_sync::TreeData;
use crate::{ForesterConfig, RpcPool};

#[derive(Debug)]
pub struct PipelineContext<T: Indexer<R>, R: RpcConnection> {
    pub indexer: Arc<Mutex<T>>,
    pub rpc_pool: RpcPool<R>,
    pub config: Arc<ForesterConfig>,
    pub tree_data: TreeData,
    pub successful_nullifications: Arc<Mutex<usize>>,
    pub rollover_state: Arc<RolloverState>,
}

impl<T: Indexer<R>, R: RpcConnection> Clone for PipelineContext<T, R> {
    fn clone(&self) -> Self {
        PipelineContext {
            indexer: Arc::clone(&self.indexer),
            rpc_pool: self.rpc_pool.clone(),
            config: Arc::clone(&self.config),
            tree_data: self.tree_data,
            successful_nullifications: Arc::clone(&self.successful_nullifications),
            rollover_state: Arc::clone(&self.rollover_state),
        }
    }
}
