use light_test_utils::rpc::rpc_connection::RpcConnection;
use rand::seq::SliceRandom;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct RpcPool<R: RpcConnection> {
    connections: Vec<Arc<Mutex<R>>>,
}

impl<R: RpcConnection> RpcPool<R> {
    pub fn new(connections: Vec<Arc<Mutex<R>>>) -> Self {
        RpcPool { connections }
    }

    pub async fn get_connection(&self) -> Arc<Mutex<R>> {
        self.connections
            .choose(&mut rand::thread_rng())
            .unwrap()
            .clone()
    }
}
