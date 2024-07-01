use clap::Parser;
use env_logger::Env;
use forester::cli::{Cli, Commands};
use forester::indexer::PhotonIndexer;
use forester::nqmt::reindex_and_store;
use forester::{
    get_address_queue_length, get_state_queue_length, init_config, init_rpc, nullify_addresses,
    nullify_state, setup_rpc, subscribe_addresses, subscribe_state, ForesterConfig, RpcPool,
};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
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
    let pool = setup_rpc(config.clone()).await;
    println!("Connection pool created");
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Subscribe) => {
            debug!(
                "Subscribe to nullify compressed accounts for indexed array: {} and merkle tree: {}",
                config.nullifier_queue_pubkey, config.state_merkle_tree_pubkey
            );

            let state_nullifier = tokio::spawn(run_subscribe_state(config.clone(), pool.clone()));
            let address_nullifier = tokio::spawn(run_subscribe_addresses(config, pool.clone()));

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
            run_nullify_state(config, pool).await;
        }
        Some(Commands::NullifyAddresses) => {
            run_nullify_addresses(config, pool).await;
        }
        Some(Commands::Nullify) => {
            let state_nullifier = tokio::spawn(run_nullify_state(config.clone(), pool.clone()));
            let address_nullifier = tokio::spawn(run_nullify_addresses(config, pool.clone()));

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
        Some(Commands::StateQueueInfo) => {
            let queue_length = get_state_queue_length(pool, config).await;
            println!("State queue length: {}", queue_length);
        }

        Some(Commands::AddressQueueInfo) => {
            let queue_length = get_address_queue_length::<SolanaRpcConnection>(pool, config).await;
            println!("Address queue length: {}", queue_length);
        }
        Some(Commands::Airdrop) => {
            init_rpc::<SolanaRpcConnection>(config.clone(), true).await;
        }
    }
}

async fn run_subscribe_state(config: Arc<ForesterConfig>, rpc_pool: RpcPool<SolanaRpcConnection>) {
    let indexer_rpc = init_rpc(config.clone(), false).await;
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
        indexer_rpc,
    )));

    subscribe_state(config.clone(), rpc_pool, indexer).await;
}

async fn run_subscribe_addresses<R: RpcConnection>(
    config: Arc<ForesterConfig>,
    rpc_pool: RpcPool<R>,
) {
    let indexer_rpc = init_rpc(config.clone(), false).await;
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
        indexer_rpc,
    )));

    subscribe_addresses(config.clone(), rpc_pool, indexer).await;
}

async fn run_nullify_state<R: RpcConnection>(config: Arc<ForesterConfig>, rpc_pool: RpcPool<R>) {
    let indexer_rpc = init_rpc(config.clone(), false).await;
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
        indexer_rpc,
    )));

    nullify_state(config.clone(), rpc_pool, indexer).await;
}

async fn run_nullify_addresses<R: RpcConnection>(
    config: Arc<ForesterConfig>,
    rpc_pool: RpcPool<R>,
) {
    let indexer_rpc = init_rpc(config.clone(), false).await;
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
        indexer_rpc,
    )));

    nullify_addresses(config.clone(), rpc_pool, indexer).await;
}
