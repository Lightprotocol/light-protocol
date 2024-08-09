pub mod cli;
pub mod config;
pub mod epoch_manager;
pub mod errors;
pub mod photon_indexer;
pub mod rollover;
pub mod rpc_pool;
pub mod settings;
pub mod tree_data_sync;
pub mod utils;

use crate::epoch_manager::{fetch_queue_item_data, run_service, WorkReport};
use crate::errors::ForesterError;
use crate::utils::get_protocol_config;
pub use config::{ForesterConfig, ForesterEpochInfo};
use env_logger::Env;
use light_test_utils::forester_epoch::{TreeAccounts, TreeType};
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use log::info;
pub use rpc_pool::RpcPool;
pub use settings::init_config;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::Signer;
use std::sync::Arc;
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
    let rpc = SolanaRpcConnection::new(config.external_services.rpc_url.to_string(), None);
    let rpc = Arc::new(Mutex::new(rpc));
    let state_trees: Vec<_> = trees
        .iter()
        .filter(|t| t.tree_type == queue_type)
        .cloned()
        .collect();

    for tree_data in state_trees {
        let queue_length = fetch_queue_item_data(rpc.clone(), &tree_data.queue)
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
) -> Result<(), ForesterError> {
    let rpc_pool = Arc::new(RpcPool::<R>::new(config.clone()).await);

    {
        let rpc = rpc_pool.get_connection().await;
        let mut rpc = rpc.lock().await;
        rpc.airdrop_lamports(&config.payer_keypair.pubkey(), LAMPORTS_PER_SOL * 100_000)
            .await
            .unwrap();
    }

    let protocol_config = {
        let rpc = rpc_pool.get_connection().await;
        Arc::new(get_protocol_config(rpc.clone()).await)
    };

    info!("Starting Forester pipeline");
    run_service(
        config,
        protocol_config,
        rpc_pool,
        indexer,
        shutdown,
        work_report_sender,
    )
    .await?;
    Ok(())
}
