pub type Result<T> = std::result::Result<T, ForesterError>;

pub mod cli;
pub mod config;
pub mod epoch_manager;
pub mod errors;
pub mod photon_indexer;
pub mod pubsub_client;
pub mod queue_helpers;
pub mod rollover;
pub mod rpc_pool;
pub mod send_transaction;
pub mod settings;
mod slot_tracker;
pub mod tree_data_sync;
pub mod utils;

use crate::epoch_manager::{run_service, WorkReport};
use crate::errors::ForesterError;
use crate::queue_helpers::fetch_queue_item_data;
use crate::rpc_pool::SolanaRpcPool;
use crate::slot_tracker::SlotTracker;
use crate::utils::get_protocol_config;
pub use config::{ForesterConfig, ForesterEpochInfo};
use env_logger::Env;
use light_test_utils::forester_epoch::{TreeAccounts, TreeType};
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use log::info;
pub use settings::init_config;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::Signer;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};

pub fn setup_logger() {
    let env = Env::new().filter_or("RUST_LOG", "info,forester=debug");
    env_logger::Builder::from_env(env).init();
}

pub async fn run_queue_info(
    config: Arc<ForesterConfig>,
    trees: Vec<TreeAccounts>,
    queue_type: TreeType,
) {
    let mut rpc = SolanaRpcConnection::new(config.external_services.rpc_url.to_string(), None);
    let state_trees: Vec<_> = trees
        .iter()
        .filter(|t| t.tree_type == queue_type)
        .cloned()
        .collect();

    for tree_data in state_trees {
        let queue_length = fetch_queue_item_data(&mut rpc, &tree_data.queue)
            .await
            .unwrap()
            .len();
        info!(
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
        config.rpc_pool_size as u32,
    )
    .await
    .map_err(|e| ForesterError::Custom(e.to_string()))?;

    {
        let mut rpc = rpc_pool.get_connection().await?;
        rpc.airdrop_lamports(&config.payer_keypair.pubkey(), LAMPORTS_PER_SOL * 100_000)
            .await?;
    }

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
        Duration::from_secs(config.slot_update_interval_seconds),
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

    info!("Starting Forester pipeline");
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
