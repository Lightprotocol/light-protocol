pub type Result<T> = std::result::Result<T, ForesterError>;

pub mod cli;
pub mod config;
pub mod epoch_manager;
pub mod errors;
pub mod forester_status;
pub mod metrics;
pub mod pagerduty;
pub mod photon_indexer;
pub mod pubsub_client;
pub mod queue_helpers;
pub mod rollover;
pub mod send_transaction;
mod slot_tracker;
pub mod telemetry;
pub mod tree_data_sync;
pub mod tree_finder;
pub mod utils;

use std::{sync::Arc, time::Duration};

use account_compression::utils::constants::{ADDRESS_QUEUE_VALUES, STATE_NULLIFIER_QUEUE_VALUES};
pub use config::{ForesterConfig, ForesterEpochInfo};
use forester_utils::{
    forester_epoch::{TreeAccounts, TreeType},
    indexer::Indexer,
};
use light_client::{
    rpc::{RpcConnection, SolanaRpcConnection},
    rpc_pool::SolanaRpcPool,
};
use solana_sdk::commitment_config::CommitmentConfig;
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::debug;

use crate::{
    epoch_manager::{run_service, WorkReport},
    errors::ForesterError,
    metrics::QUEUE_LENGTH,
    queue_helpers::fetch_queue_item_data,
    slot_tracker::SlotTracker,
    utils::get_protocol_config,
};

pub async fn run_queue_info(
    config: Arc<ForesterConfig>,
    trees: Vec<TreeAccounts>,
    queue_type: TreeType,
) {
    let mut rpc = SolanaRpcConnection::new(config.external_services.rpc_url.to_string(), None);
    let trees: Vec<_> = trees
        .iter()
        .filter(|t| t.tree_type == queue_type)
        .cloned()
        .collect();

    for tree_data in trees {
        let length = if tree_data.tree_type == TreeType::State {
            STATE_NULLIFIER_QUEUE_VALUES
        } else {
            ADDRESS_QUEUE_VALUES
        };

        let queue_length = fetch_queue_item_data(&mut rpc, &tree_data.queue, 0, length, length)
            .await
            .unwrap()
            .len();
        QUEUE_LENGTH
            .with_label_values(&[&*queue_type.to_string(), &tree_data.merkle_tree.to_string()])
            .set(queue_length as i64);
        println!(
            "{:?} queue {} length: {}",
            queue_type, tree_data.queue, queue_length
        );
    }
}

pub async fn run_pipeline<R: RpcConnection, I: Indexer<R>>(
    config: Arc<ForesterConfig>,
    indexer: Arc<Mutex<I>>,
    shutdown: oneshot::Receiver<()>,
    work_report_sender: mpsc::Sender<WorkReport>,
) -> Result<()> {
    let rpc_pool = SolanaRpcPool::<R>::new(
        config.external_services.rpc_url.to_string(),
        CommitmentConfig::confirmed(),
        config.general_config.rpc_pool_size as u32,
    )
    .await
    .map_err(|e| ForesterError::Custom(e.to_string()))?;

    let protocol_config = {
        let mut rpc = rpc_pool.get_connection().await?;
        get_protocol_config(&mut *rpc).await
    };

    let arc_pool = Arc::new(rpc_pool);
    let arc_pool_clone = Arc::clone(&arc_pool);

    let slot = {
        let mut rpc = arc_pool.get_connection().await?;
        rpc.get_slot().await?
    };
    let slot_tracker = SlotTracker::new(
        slot,
        Duration::from_secs(config.general_config.slot_update_interval_seconds),
    );
    let arc_slot_tracker = Arc::new(slot_tracker);
    let arc_slot_tracker_clone = arc_slot_tracker.clone();
    tokio::spawn(async move {
        let mut rpc = arc_pool_clone
            .get_connection()
            .await
            .expect("Failed to get RPC connection");
        SlotTracker::run(arc_slot_tracker_clone, &mut *rpc).await;
    });

    debug!("Starting Forester pipeline");
    run_service(
        config,
        Arc::new(protocol_config),
        arc_pool,
        indexer,
        shutdown,
        work_report_sender,
        arc_slot_tracker,
    )
    .await?;
    Ok(())
}
