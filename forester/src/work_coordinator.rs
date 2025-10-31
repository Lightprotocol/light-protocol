use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use light_compressed_account::QueueType;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tracing::{debug, error, info, trace, warn};

// Generated protobuf code
pub mod proto {
    tonic::include_proto!("photon");
}

use proto::{queue_service_client::QueueServiceClient, SubscribeQueueUpdatesRequest};

/// Message sent to tree tasks when queue updates occur
#[derive(Debug, Clone)]
pub struct QueueUpdateMessage {
    pub tree: Pubkey,
    pub queue: Pubkey,
    pub queue_type: QueueType,
    pub queue_size: u64,
    pub slot: u64,
    pub update_type: proto::UpdateType,
}

#[derive(Debug)]
pub struct WorkCoordinator {
    grpc_client: RwLock<QueueServiceClient<Channel>>,
    tree_notifiers: Arc<RwLock<HashMap<Pubkey, mpsc::Sender<QueueUpdateMessage>>>>,
    connection_healthy: Arc<AtomicBool>,
    photon_grpc_url: String,
}

impl WorkCoordinator {
    pub async fn new(photon_grpc_url: String) -> Result<Self> {
        info!("Connecting to Photon gRPC at {}", photon_grpc_url);

        let grpc_client = QueueServiceClient::connect(photon_grpc_url.clone())
            .await
            .context("Failed to connect to Photon gRPC service")?;

        info!("Successfully connected to Photon gRPC");

        Ok(Self {
            grpc_client: RwLock::new(grpc_client),
            tree_notifiers: Arc::new(RwLock::new(HashMap::new())),
            connection_healthy: Arc::new(AtomicBool::new(false)),
            photon_grpc_url,
        })
    }

    pub async fn register_tree(&self, tree_pubkey: Pubkey) -> mpsc::Receiver<QueueUpdateMessage> {
        let (tx, rx) = mpsc::channel(100);
        self.tree_notifiers.write().await.insert(tree_pubkey, tx);
        debug!("Registered tree {} for queue updates", tree_pubkey);
        rx
    }

    pub async fn unregister_tree(&self, tree_pubkey: &Pubkey) {
        self.tree_notifiers.write().await.remove(tree_pubkey);
        debug!("Unregistered tree {}", tree_pubkey);
    }

    pub async fn run_dispatcher(self: Arc<Self>) -> Result<()> {
        let mut reconnect_delay = Duration::from_secs(1);
        const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(30);

        loop {
            match self.dispatch_loop().await {
                Ok(()) => {
                    warn!("gRPC stream ended; attempting to reconnect…");
                    self.connection_healthy.store(false, Ordering::Relaxed);
                    tokio::time::sleep(reconnect_delay).await;
                    let _ = self.reconnect().await;
                    reconnect_delay = Duration::from_secs(1);
                    continue;
                }
                Err(e) => {
                    error!("gRPC dispatcher error: {:?}", e);
                    self.connection_healthy.store(false, Ordering::Relaxed);

                    warn!("Reconnecting in {:?}...", reconnect_delay);
                    tokio::time::sleep(reconnect_delay).await;
                    reconnect_delay = std::cmp::min(reconnect_delay * 2, MAX_RECONNECT_DELAY);

                    match self.reconnect().await {
                        Ok(()) => {
                            info!("Successfully reconnected to Photon gRPC");
                            reconnect_delay = Duration::from_secs(1);
                        }
                        Err(e) => {
                            error!("Failed to reconnect: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    async fn reconnect(&self) -> Result<()> {
        let new_client = QueueServiceClient::connect(self.photon_grpc_url.clone())
            .await
            .context("Failed to reconnect to Photon gRPC service")?;
        *self.grpc_client.write().await = new_client;
        Ok(())
    }

    async fn dispatch_loop(&self) -> Result<()> {
        info!("Starting gRPC queue update subscription");

        let request = SubscribeQueueUpdatesRequest {
            trees: vec![],
            send_initial_state: true,
        };

        let mut stream = self
            .grpc_client
            .read()
            .await
            .clone()
            .subscribe_queue_updates(request)
            .await
            .context("Failed to subscribe to queue updates")?
            .into_inner();

        self.connection_healthy.store(true, Ordering::Relaxed);
        info!("gRPC subscription established successfully");

        while let Some(update_result) = stream.next().await {
            let update = update_result.context("Error receiving queue update")?;

            let queue_info = update
                .queue_info
                .ok_or_else(|| anyhow!("Missing queue_info in update"))?;

            let tree_pubkey = queue_info
                .tree
                .parse::<Pubkey>()
                .context("Failed to parse tree pubkey")?;

            let queue_pubkey = queue_info
                .queue
                .parse::<Pubkey>()
                .context("Failed to parse queue pubkey")?;

            let queue_type = QueueType::from(queue_info.queue_type as u64);

            let update_type = proto::UpdateType::try_from(update.update_type)
                .unwrap_or(proto::UpdateType::Unspecified);

            let message = QueueUpdateMessage {
                tree: tree_pubkey,
                queue: queue_pubkey,
                queue_type,
                queue_size: queue_info.queue_size,
                slot: update.slot,
                update_type,
            };

            let notifiers = self.tree_notifiers.read().await;
            if let Some(tx) = notifiers.get(&tree_pubkey) {
                match tx.try_send(message.clone()) {
                    Ok(()) => {
                        trace!(
                            "Routed update to tree {}: {} items (type: {:?})",
                            tree_pubkey,
                            message.queue_size,
                            queue_type
                        );
                    }
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        warn!(
                            "Tree {} channel full, dropping update (tree processing slower than updates)",
                            tree_pubkey
                        );
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        debug!("Tree {} channel closed (task likely finished)", tree_pubkey);
                    }
                }
            } else {
                trace!("Received update for unregistered tree {}", tree_pubkey);
            }
        }

        warn!("gRPC stream ended");
        self.connection_healthy.store(false, Ordering::Relaxed);
        Ok(())
    }

    pub fn is_healthy(&self) -> bool {
        self.connection_healthy.load(Ordering::Relaxed)
    }

    pub async fn registered_tree_count(&self) -> usize {
        self.tree_notifiers.read().await.len()
    }

    pub async fn shutdown(&self) {
        info!("Shutting down WorkCoordinator");
        self.connection_healthy.store(false, Ordering::Relaxed);
    }
}
