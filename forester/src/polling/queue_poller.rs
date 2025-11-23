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
    tree_notifiers: HashMap<Pubkey, Vec<mpsc::Sender<QueueUpdateMessage>>>,
    polling_active: Arc<AtomicBool>,
}

impl Actor for QueueInfoPoller {
    type Args = Self;
    type Error = anyhow::Error;

    async fn on_start(state: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self> {
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
        self.polling_active.store(false, Ordering::Release);
        Ok(())
    }
}

impl QueueInfoPoller {
    pub fn new(indexer_url: String, api_key: Option<String>) -> Self {
        let indexer = PhotonIndexer::new(indexer_url, api_key);

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

                let queue_infos: Vec<QueueInfo> = result
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

                debug!(
                    "Indexer returned {} queues at slot {} (trees: {:?})",
                    queue_infos.len(),
                    slot,
                    queue_infos
                        .iter()
                        .map(|q| format!("{}:{}", q.tree, q.queue_size))
                        .collect::<Vec<_>>()
                );

                Ok(queue_infos)
            }
            Err(e) => {
                error!("Failed to call getQueueInfo: {:?}", e);
                Err(anyhow::anyhow!("Failed to call getQueueInfo").context(e))
            }
        }
    }

    fn distribute_updates(&mut self, queue_infos: Vec<QueueInfo>) {
        let registered_trees: Vec<Pubkey> = self.tree_notifiers.keys().cloned().collect();
        let mut matched_count = 0;
        let mut unmatched_trees = Vec::new();

        for info in queue_infos {
            if let Some(senders) = self.tree_notifiers.get_mut(&info.tree) {
                matched_count += 1;
                let message = QueueUpdateMessage {
                    tree: info.tree,
                    queue: info.queue,
                    queue_type: info.queue_type,
                    queue_size: info.queue_size,
                    slot: info.slot,
                };

                // Track which senders to remove (closed channels)
                let mut closed_indices = Vec::new();

                for (idx, tx) in senders.iter().enumerate() {
                    match tx.try_send(message.clone()) {
                        Ok(()) => {
                            debug!(
                                "Routed update to tree {}: {} items (type: {:?})",
                                info.tree,
                                message.queue_size,
                                info.queue_type
                            );
                        }
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            debug!(
                                "Tree {} channel full, dropping update",
                                info.tree
                            );
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => {
                            trace!("Tree {} channel {} closed", info.tree, idx);
                            closed_indices.push(idx);
                        }
                    }
                }

                for idx in closed_indices.into_iter().rev() {
                    senders.swap_remove(idx);
                }
            } else {
                unmatched_trees.push((info.tree, info.queue_size));
            }
        }

        if !unmatched_trees.is_empty() {
            debug!(
                "Indexer returned {} trees not registered with poller: {:?}. Registered trees: {:?}",
                unmatched_trees.len(),
                unmatched_trees,
                registered_trees
            );
        }

        if matched_count == 0 && !registered_trees.is_empty() {
            warn!(
                "No queue updates matched registered trees! Registered: {:?}",
                registered_trees
            );
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

        let senders = self.tree_notifiers.entry(msg.tree_pubkey).or_default();
        let sender_count = senders.len();
        senders.push(tx);

        if sender_count > 0 {
            debug!(
                "Added concurrent registration for tree {} (now {} receivers)",
                msg.tree_pubkey,
                sender_count + 1
            );
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
        if let Some(senders) = self.tree_notifiers.remove(&msg.tree_pubkey) {
            debug!(
                "Unregistered tree {} ({} senders removed)",
                msg.tree_pubkey,
                senders.len()
            );
            drop(senders);
        } else {
            warn!(
                "Attempted to unregister non-existent tree {}",
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
        self.tree_notifiers
            .values()
            .filter(|senders| !senders.is_empty())
            .count()
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
        self.tree_notifiers.retain(|_, senders| !senders.is_empty());

        if self.tree_notifiers.is_empty() {
            return;
        }

        let num_registered = self.tree_notifiers.len();
        debug!(
            "Polling queue info for {} registered trees",
            num_registered
        );

        match self.poll_queue_info().await {
            Ok(queue_infos) => {
                if queue_infos.is_empty() {
                    debug!(
                        "Indexer returned empty queue list (0 queues) for {} registered trees",
                        num_registered
                    );
                }
                self.distribute_updates(queue_infos);
            }
            Err(e) => {
                error!("Failed to poll queue info: {:?}", e);
            }
        }
    }
}

async fn polling_loop(actor_ref: ActorRef<QueueInfoPoller>, polling_active: Arc<AtomicBool>) {
    let mut interval = tokio::time::interval(Duration::from_secs(POLLING_INTERVAL_SECS));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        if !polling_active.load(Ordering::Acquire) {
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

        interval.tick().await;

        if !polling_active.load(Ordering::Acquire) {
            break;
        }
    }
}
