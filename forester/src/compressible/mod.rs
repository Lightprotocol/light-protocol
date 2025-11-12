pub mod config;
pub mod types;
pub mod state;
pub mod subscriber;
pub mod compressor;

pub use config::CompressibleConfig;
pub use types::CompressibleAccountState;
pub use state::CompressibleAccountTracker;

use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

use forester_utils::rpc_pool::SolanaRpcPool;
use light_client::rpc::Rpc;
use solana_sdk::signature::Keypair;

use subscriber::AccountSubscriber;
use compressor::Compressor;

/// Main compressible service that orchestrates account subscription and compression
pub struct CompressibleService<R: Rpc> {
    config: CompressibleConfig,
    rpc_pool: Arc<SolanaRpcPool<R>>,
    payer_keypair: Keypair,
}

impl<R: Rpc> CompressibleService<R> {
    pub fn new(
        config: CompressibleConfig,
        rpc_pool: Arc<SolanaRpcPool<R>>,
        payer_keypair: Keypair,
    ) -> Self {
        Self {
            config,
            rpc_pool,
            payer_keypair,
        }
    }

    pub async fn run(self) -> crate::Result<()> {
        info!("Starting Compressible Service");

        let tracker = Arc::new(CompressibleAccountTracker::new());

        let (_shutdown_tx, shutdown_rx) = mpsc::channel(1);

        // Spawn subscriber
        let subscriber_tracker = tracker.clone();
        let ws_url = self.config.ws_url.clone();
        let subscriber_handle = tokio::spawn(async move {
            let mut subscriber = AccountSubscriber::new(ws_url, subscriber_tracker, shutdown_rx);
            if let Err(e) = subscriber.run().await {
                error!("Subscriber error: {:?}", e);
            }
        });

        // Spawn compressor
        let compressor_tracker = tracker.clone();
        let rpc_pool = self.rpc_pool.clone();
        let payer_keypair = self.payer_keypair;
        let compressor_handle = tokio::spawn(async move {
            let mut compressor = Compressor::new(rpc_pool, compressor_tracker, payer_keypair);
            if let Err(e) = compressor.run().await {
                error!("Compressor error: {:?}", e);
            }
        });

        // Wait for both tasks
        let result = tokio::try_join!(subscriber_handle, compressor_handle);

        if let Err(e) = result {
            error!("Compressible service task error: {:?}", e);
        }

        info!("Compressible Service stopped");
        Ok(())
    }
}
