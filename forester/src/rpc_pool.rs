use crate::ForesterConfig;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use rand::seq::SliceRandom;
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_sdk::signature::Signer;
use std::sync::Arc;
use tokio::sync::Mutex;

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
            let rpc = init_rpc::<R>(config.clone(), false).await;
            let rpc = Arc::new(Mutex::new(rpc));
            connections.push(rpc);
        }
        Self { connections }
    }
}

pub async fn init_rpc<R: RpcConnection>(config: Arc<ForesterConfig>, airdrop: bool) -> R {
    let mut rpc = R::new(
        config.external_services.rpc_url.clone(),
        Some(CommitmentConfig {
            commitment: CommitmentLevel::Confirmed,
        }),
    );

    if airdrop {
        rpc.airdrop_lamports(&config.payer_keypair.pubkey(), 10_000_000_000)
            .await
            .unwrap();
    }

    rpc
}
