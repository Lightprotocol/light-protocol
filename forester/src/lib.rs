pub type Result<T> = anyhow::Result<T>;

pub mod cli;
pub mod config;
pub mod epoch_manager;
pub mod errors;
pub mod forester_status;
pub mod helius_priority_fee_types;
mod indexer_type;
pub mod metrics;
pub mod pagerduty;
pub mod processor;
pub mod pubsub_client;
pub mod queue_helpers;
pub mod rollover;
mod slot_tracker;
pub mod smart_transaction;
pub mod telemetry;
pub mod tree_data_sync;
pub mod tree_finder;
pub mod utils;

use std::{sync::Arc, time::Duration};

use account_compression::utils::constants::{ADDRESS_QUEUE_VALUES, STATE_NULLIFIER_QUEUE_VALUES};
pub use config::{ForesterConfig, ForesterEpochInfo};
use forester_utils::{
    forester_epoch::TreeAccounts, rate_limiter::RateLimiter, rpc_pool::SolanaRpcPoolBuilder,
};
use light_client::{
    indexer::Indexer,
    rpc::{LightClient, LightClientConfig, Rpc},
};
use light_compressed_account::TreeType;
use solana_sdk::commitment_config::CommitmentConfig;
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::debug;

use crate::{
    epoch_manager::{run_service, WorkReport},
    indexer_type::IndexerType,
    metrics::QUEUE_LENGTH,
    processor::tx_cache::ProcessedHashCache,
    queue_helpers::fetch_queue_item_data,
    slot_tracker::SlotTracker,
    utils::get_protocol_config,
};

pub async fn run_queue_info(
    config: Arc<ForesterConfig>,
    trees: Vec<TreeAccounts>,
    queue_type: TreeType,
) {
    let mut rpc = LightClient::new(LightClientConfig {
        url: config.external_services.rpc_url.to_string(),
        commitment_config: None,
        fetch_active_tree: false,

        with_indexer: false,
    })
    .await
    .unwrap();
    let trees: Vec<_> = trees
        .iter()
        .filter(|t| t.tree_type == queue_type)
        .cloned()
        .collect();

    for tree_data in trees {
        if tree_data.tree_type == TreeType::StateV2 {
            continue;
        }

        let length = if tree_data.tree_type == TreeType::StateV1 {
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

pub async fn run_pipeline<R: Rpc, I: Indexer + IndexerType<R> + 'static>(
    config: Arc<ForesterConfig>,
    rpc_rate_limiter: Option<RateLimiter>,
    send_tx_rate_limiter: Option<RateLimiter>,
    indexer: Arc<Mutex<I>>,
    shutdown: oneshot::Receiver<()>,
    work_report_sender: mpsc::Sender<WorkReport>,
) -> Result<()> {
    let mut builder = SolanaRpcPoolBuilder::<R>::default()
        .url(config.external_services.rpc_url.to_string())
        .commitment(CommitmentConfig::confirmed())
        .max_size(config.rpc_pool_config.max_size)
        .connection_timeout_secs(config.rpc_pool_config.connection_timeout_secs)
        .idle_timeout_secs(config.rpc_pool_config.idle_timeout_secs)
        .max_retries(config.rpc_pool_config.max_retries)
        .initial_retry_delay_ms(config.rpc_pool_config.initial_retry_delay_ms)
        .max_retry_delay_ms(config.rpc_pool_config.max_retry_delay_ms);

    if let Some(limiter) = rpc_rate_limiter {
        builder = builder.rpc_rate_limiter(limiter);
    }

    if let Some(limiter) = send_tx_rate_limiter {
        builder = builder.send_tx_rate_limiter(limiter);
    }

    let rpc_pool = builder.build().await?;

    let protocol_config = {
        let mut rpc = rpc_pool.get_connection().await?;
        get_protocol_config(&mut *rpc).await
    };

    let arc_pool = Arc::new(rpc_pool);
    let arc_pool_clone = Arc::clone(&arc_pool);

    let slot = {
        let rpc = arc_pool.get_connection().await?;
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

    let tx_cache = Arc::new(Mutex::new(ProcessedHashCache::new(15)));

    debug!("Starting Forester pipeline");
    run_service(
        config,
        Arc::new(protocol_config),
        arc_pool,
        indexer,
        shutdown,
        work_report_sender,
        arc_slot_tracker,
        tx_cache,
    )
    .await?;
    Ok(())
}
