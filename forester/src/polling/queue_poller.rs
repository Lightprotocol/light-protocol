use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Result;
use kameo::{
    actor::{ActorRef, WeakActorRef},
    error::ActorStopReason,
    message::Message,
    Actor,
};
use light_client::indexer::{photon_indexer::PhotonIndexer, Indexer};
use light_compressed_account::QueueType;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

const POLLING_INTERVAL_SECS: u64 = 1;

#[derive(Debug, Clone)]
pub struct QueueUpdateMessage {
    pub tree: Pubkey,
    pub queue: Pubkey,
    pub queue_type: QueueType,
    pub queue_size: u64,
    pub slot: u64,
}

pub struct QueueInfoPoller {
    indexer: PhotonIndexer,
    tree_notifiers: HashMap<Pubkey, mpsc::Sender<QueueUpdateMessage>>,
    polling_active: Arc<AtomicBool>,
}

impl Actor for QueueInfoPoller {
    type Args = Self;
    type Error = anyhow::Error;

    async fn on_start(state: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self> {
        info!("QueueInfoPoller actor starting");

        let polling_active = state.polling_active.clone();
        tokio::spawn(async move {
            polling_loop(actor_ref, polling_active).await;
        });

        Ok(state)
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<()> {
        info!("QueueInfoPoller actor stopping");
        // Use Release ordering to synchronize with Acquire loads in polling_loop
        self.polling_active.store(false, Ordering::Release);
        Ok(())
    }
}

impl QueueInfoPoller {
    pub fn new(indexer_url: String, api_key: Option<String>) -> Self {
        let indexer = PhotonIndexer::new(format!("{}/v1", indexer_url), api_key);

        Self {
            indexer,
            tree_notifiers: HashMap::new(),
            polling_active: Arc::new(AtomicBool::new(true)),
        }
    }

    async fn poll_queue_info(&mut self) -> Result<Vec<QueueInfo>> {
        match self.indexer.get_queue_info(None).await {
            Ok(response) => {
                let result = response.value;
                let slot = result.slot;

                let queue_infos = result
                    .queues
                    .into_iter()
                    .map(|queue| QueueInfo {
                        tree: queue.tree,
                        queue: queue.queue,
                        queue_type: QueueType::from(queue.queue_type as u64),
                        queue_size: queue.queue_size,
                        slot,
                    })
                    .collect();

                Ok(queue_infos)
            }
            Err(e) => {
                error!("Failed to call getQueueInfo: {:?}", e);
                Err(anyhow::anyhow!("Failed to call getQueueInfo").context(e))
            }
        }
    }

    fn distribute_updates(&self, queue_infos: Vec<QueueInfo>) {
        for info in queue_infos {
            if let Some(tx) = self.tree_notifiers.get(&info.tree) {
                let message = QueueUpdateMessage {
                    tree: info.tree,
                    queue: info.queue,
                    queue_type: info.queue_type,
                    queue_size: info.queue_size,
                    slot: info.slot,
                };

                match tx.try_send(message.clone()) {
                    Ok(()) => {
                        trace!(
                            "Routed update to tree {}: {} items (type: {:?})",
                            info.tree,
                            message.queue_size,
                            info.queue_type
                        );
                    }
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        debug!(
                            "Tree {} channel full, dropping update (tree processing slower than updates)",
                            info.tree
                        );
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        trace!("Tree {} channel closed (task likely finished)", info.tree);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct QueueInfo {
    tree: Pubkey,
    queue: Pubkey,
    queue_type: QueueType,
    queue_size: u64,
    slot: u64,
}

#[derive(Debug, Clone)]
pub struct RegisterTree {
    pub tree_pubkey: Pubkey,
}

impl Message<RegisterTree> for QueueInfoPoller {
    type Reply = mpsc::Receiver<QueueUpdateMessage>;

    async fn handle(
        &mut self,
        msg: RegisterTree,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let (tx, rx) = mpsc::channel(1000);

        // Check if there's already a sender registered for this tree
        if let Some(old_sender) = self.tree_notifiers.insert(msg.tree_pubkey, tx) {
            warn!(
                "Double registration detected for tree {}. Replacing existing sender (previous receiver will be closed).",
                msg.tree_pubkey
            );
            // The old sender is dropped here, which will close the old receiver
            drop(old_sender);
        } else {
            debug!("Registered tree {} for queue updates", msg.tree_pubkey);
        }

        rx
    }
}

#[derive(Debug, Clone)]
pub struct UnregisterTree {
    pub tree_pubkey: Pubkey,
}

impl Message<UnregisterTree> for QueueInfoPoller {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: UnregisterTree,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // Check if the tree was actually registered before removing
        if let Some(sender) = self.tree_notifiers.remove(&msg.tree_pubkey) {
            debug!("Unregistered tree {}", msg.tree_pubkey);
            // Drop the sender to close the receiver
            drop(sender);
        } else {
            warn!(
                "Attempted to unregister non-existent tree {}. This may indicate a mismatch between receiver drops and explicit unregistration.",
                msg.tree_pubkey
            );
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RegisteredTreeCount;

impl Message<RegisteredTreeCount> for QueueInfoPoller {
    type Reply = usize;

    async fn handle(
        &mut self,
        _msg: RegisteredTreeCount,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.tree_notifiers.len()
    }
}

#[derive(Debug, Clone, Copy)]
struct PollNow;

impl Message<PollNow> for QueueInfoPoller {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: PollNow,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.tree_notifiers.is_empty() {
            debug!("No trees registered; skipping queue info poll");
            return;
        }

        match self.poll_queue_info().await {
            Ok(queue_infos) => {
                self.distribute_updates(queue_infos);
            }
            Err(e) => {
                error!("Failed to poll queue info: {:?}", e);
            }
        }
    }
}

async fn polling_loop(actor_ref: ActorRef<QueueInfoPoller>, polling_active: Arc<AtomicBool>) {
    info!("Starting queue info polling loop (1 second interval)");

    let mut interval = tokio::time::interval(Duration::from_secs(POLLING_INTERVAL_SECS));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        // Check if polling should continue
        if !polling_active.load(Ordering::Acquire) {
            info!("Polling loop shutting down (polling_active=false)");
            break;
        }

        interval.tick().await;

        // Check again after the tick in case shutdown was signaled during sleep
        if !polling_active.load(Ordering::Acquire) {
            info!("Polling loop shutting down (polling_active=false)");
            break;
        }

        match actor_ref.tell(PollNow).send().await {
            Ok(_) => {}
            Err(e) => {
                if polling_active.load(Ordering::Acquire) {
                    error!("Failed to send poll message to actor: {:?}", e);
                } else {
                    info!("Poll message send failed during shutdown: {:?}", e);
                }
                break;
            }
        }
    }

    info!("Polling loop stopped");
}
