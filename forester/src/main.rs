use clap::Parser;
use env_logger::Env;
use forester::cli::{Cli, Commands};
use forester::indexer::PhotonIndexer;
use forester::nqmt::reindex_and_store;
use forester::{
    init_config, init_rpc, nullify_addresses, nullify_state, subscribe_addresses, subscribe_state, ForesterConfig
};
use log::{debug, error};
use std::sync::Arc;

fn setup_logger() {
    let env = Env::new().filter_or("RUST_LOG", "info,forester=debug");
    env_logger::Builder::from_env(env).init();
}

#[tokio::main]
async fn main() {
    setup_logger();
    let config: Arc<ForesterConfig> = Arc::new(init_config());
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Subscribe) => {
            debug!(
                "Subscribe to nullify compressed accounts for indexed array: {} and merkle tree: {}",
                config.nullifier_queue_pubkey, config.state_merkle_tree_pubkey
            );
            
            let state_nullifier = tokio::spawn(run_subscribe_state(config.clone()));
            let address_nullifier = tokio::spawn(run_subscribe_addresses(config));

            // Wait for both nullifiers to complete
            let (state_result, address_result) = tokio::join!(state_nullifier, address_nullifier);

            if let Err(e) = state_result {
                error!("State nullifier encountered an error: {:?}", e);
            }

            if let Err(e) = address_result {
                error!("Address nullifier encountered an error: {:?}", e);
            }

            debug!("All nullification processes completed");

        }
        Some(Commands::NullifyState) => {
            run_nullify_state(config).await;
        }
        Some(Commands::NullifyAddresses) => {
            run_nullify_addresses(config).await;
        }
        Some(Commands::Nullify) => {
            let state_nullifier = tokio::spawn(run_nullify_state(config.clone()));
            let address_nullifier = tokio::spawn(run_nullify_addresses(config));

            // Wait for both nullifiers to complete
            let (state_result, address_result) = tokio::join!(state_nullifier, address_nullifier);

            if let Err(e) = state_result {
                error!("State nullifier encountered an error: {:?}", e);
            }

            if let Err(e) = address_result {
                error!("Address nullifier encountered an error: {:?}", e);
            }

            debug!("All nullification processes completed");
        }
        Some(Commands::Index) => {
            debug!("Reindex merkle tree & nullifier queue accounts");
            debug!(
                "Initial merkle tree account: {}",
                config.state_merkle_tree_pubkey
            );
            let _ = reindex_and_store(&config);
        }
        None => {
            return;
        }
    }
}

async fn run_subscribe_state(config: Arc<ForesterConfig>) {
    let rpc = init_rpc(config.clone(), false).await;
    let rpc = Arc::new(tokio::sync::Mutex::new(rpc));

    let indexer_rpc = init_rpc(config.clone(), false).await;
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
        indexer_rpc,
    )));

    subscribe_state(config.clone(), rpc, indexer).await;
}

async fn run_subscribe_addresses(config: Arc<ForesterConfig>) {
    let rpc = init_rpc(config.clone(), false).await;
    let rpc = Arc::new(tokio::sync::Mutex::new(rpc));

    let indexer_rpc = init_rpc(config.clone(), false).await;
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
        indexer_rpc,
    )));

    subscribe_addresses(config.clone(), rpc, indexer).await;
}

async fn run_nullify_state(config: Arc<ForesterConfig>) {
    let rpc = init_rpc(config.clone(), false).await;
    let rpc = Arc::new(tokio::sync::Mutex::new(rpc));

    let indexer_rpc = init_rpc(config.clone(), false).await;
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
        indexer_rpc,
    )));

    nullify_state(config.clone(), rpc, indexer).await;
}

async fn run_nullify_addresses(config: Arc<ForesterConfig>) {
    let rpc = init_rpc(config.clone(), false).await;
    let rpc = Arc::new(tokio::sync::Mutex::new(rpc));

    let indexer_rpc = init_rpc(config.clone(), false).await;
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
        indexer_rpc,
    )));

    nullify_addresses(config.clone(), rpc, indexer).await;
}
