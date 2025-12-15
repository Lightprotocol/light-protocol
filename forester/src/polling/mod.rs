pub mod onchain_queue_poller;
pub mod queue_poller;

use kameo::actor::ActorRef;
use light_client::rpc::Rpc;
use light_compressed_account::TreeType;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::mpsc;
use tracing::{debug, info};

pub use onchain_queue_poller::{
    OnChainQueuePoller, RegisterTreeOnChain, RegisteredTreeCountOnChain, UnregisterTreeOnChain,
};
pub use queue_poller::{
    QueueInfoPoller, QueueUpdateMessage, RegisterTree, RegisteredTreeCount, UnregisterTree,
};

/// Wrapper enum to abstract over different queue polling implementations.
pub enum QueuePollerRef<R: Rpc + 'static> {
    /// Polls queue info from an indexer API (Photon)
    Indexer(ActorRef<QueueInfoPoller>),
    /// Polls queue info directly from on-chain accounts
    OnChain(ActorRef<OnChainQueuePoller<R>>),
}

impl<R: Rpc + 'static> std::fmt::Debug for QueuePollerRef<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueuePollerRef::Indexer(_) => f.debug_tuple("Indexer").field(&"...").finish(),
            QueuePollerRef::OnChain(_) => f.debug_tuple("OnChain").field(&"...").finish(),
        }
    }
}

impl<R: Rpc + 'static> Clone for QueuePollerRef<R> {
    fn clone(&self) -> Self {
        match self {
            QueuePollerRef::Indexer(ref actor) => QueuePollerRef::Indexer(actor.clone()),
            QueuePollerRef::OnChain(ref actor) => QueuePollerRef::OnChain(actor.clone()),
        }
    }
}

impl<R: Rpc + 'static> QueuePollerRef<R> {
    /// Register a tree for queue polling updates.
    /// Returns a receiver for queue update messages.
    pub async fn register_tree(
        &self,
        tree_pubkey: Pubkey,
        queue_pubkey: Pubkey,
        tree_type: TreeType,
        min_queue_size: u64,
    ) -> anyhow::Result<mpsc::Receiver<QueueUpdateMessage>> {
        match self {
            QueuePollerRef::Indexer(actor) => {
                debug!(
                    "Registering tree {} with indexer poller (min_queue_size={})",
                    tree_pubkey, min_queue_size
                );
                actor
                    .ask(RegisterTree {
                        tree_pubkey,
                        min_queue_size,
                    })
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to register tree: {:?}", e))
            }
            QueuePollerRef::OnChain(actor) => {
                debug!(
                    "Registering tree {} with on-chain poller (type={:?}, min_queue_size={})",
                    tree_pubkey, tree_type, min_queue_size
                );
                actor
                    .ask(RegisterTreeOnChain {
                        tree_pubkey,
                        queue_pubkey,
                        tree_type,
                        min_queue_size,
                    })
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to register tree: {:?}", e))
            }
        }
    }

    /// Log information about the poller type
    pub fn log_info(&self) {
        match self {
            QueuePollerRef::Indexer(_) => {
                info!("Using indexer-based queue polling");
            }
            QueuePollerRef::OnChain(_) => {
                info!("Using on-chain queue polling (reading directly from RPC)");
            }
        }
    }
}
