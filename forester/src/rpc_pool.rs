use std::sync::Arc;

use rand::seq::SliceRandom;
use solana_sdk::commitment_config::CommitmentConfig;
use tokio::sync::Mutex;

use light_test_utils::rpc::rpc_connection::RpcConnection;

use crate::ForesterConfig;

#[derive(Debug, Clone)]
pub struct RpcPool<R: RpcConnection> {
    connections: Vec<Arc<Mutex<R>>>,
}

impl<R: RpcConnection> RpcPool<R> {
    pub async fn get_connection(&self) -> Arc<Mutex<R>> {
        self.connections
            .choose(&mut rand::thread_rng())
            .unwrap()
            .clone()
    }

    pub async fn new(config: Arc<ForesterConfig>) -> RpcPool<R> {
        let mut connections: Vec<Arc<Mutex<R>>> = Vec::new();
        for _ in 0..20 {
            let rpc = R::new(
                &config.external_services.rpc_url,
                Some(CommitmentConfig::confirmed()),
            );
            let rpc = Arc::new(Mutex::new(rpc));
            connections.push(rpc);
        }
        Self { connections }
    }
}
