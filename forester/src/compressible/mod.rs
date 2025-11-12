pub mod config;
pub mod state;
pub mod subscriber;
pub mod types;

pub use config::CompressibleConfig;
pub use state::CompressibleAccountTracker;
pub use types::CompressibleAccountState;

use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{error, info};

use forester_utils::rpc_pool::SolanaRpcPool;
use light_client::rpc::Rpc;
use solana_sdk::signature::Keypair;

use subscriber::AccountSubscriber;

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

    pub async fn run(self, shutdown: oneshot::Receiver<()>) -> crate::Result<()> {
        info!("Starting Compressible Service");

        let tracker = Arc::new(CompressibleAccountTracker::new());

        // Spawn subscriber with shutdown signal
        let subscriber_tracker = tracker.clone();
        let ws_url = self.config.ws_url.clone();
        let subscriber_handle = tokio::spawn(async move {
            let mut subscriber = AccountSubscriber::new(ws_url, subscriber_tracker, shutdown);
            if let Err(e) = subscriber.run().await {
                error!("Subscriber error: {:?}", e);
            }
        });

        // Wait for subscriber to complete (it will exit on shutdown signal)
        if let Err(e) = subscriber_handle.await {
            error!("Subscriber task failed: {:?}", e);
        }

        info!("Compressible Service stopped");
        Ok(())
    }
}
