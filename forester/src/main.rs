use clap::Parser;
use env_logger::Env;
use forester::cli::{Cli, Commands};
use forester::indexer::PhotonIndexer;
use forester::rollover::RolloverState;
use forester::tree_sync::{fetch_trees, serialize_tree_data, TreeData};
use forester::{
    get_address_queue_length, get_state_queue_length, init_config, nullify_addresses,
    nullify_state, subscribe_addresses, subscribe_state, ForesterConfig, RpcPool, TreeType,
};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use log::error;
use log::{debug, info, warn};
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signer::Signer;
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

    info!("Fetching trees...");
    let trees = fetch_trees(&config.external_services.rpc_url).await;
    if trees.is_empty() {
        warn!("No trees found. Exiting.");
        return;
    }

    match &cli.command {
        Some(Commands::Subscribe) => {
            run_subscribe(
                config.clone(),
                RpcPool::<SolanaRpcConnection>::new(config.clone()).await,
                Arc::new(RolloverState::new()),
                trees,
            )
            .await;
        }
        Some(Commands::NullifyState) => {
            let state_trees: Vec<_> = trees
                .iter()
                .filter(|&t| matches!(t.tree_type, TreeType::State))
                .cloned()
                .collect();
            info!("State trees length: {:?}", state_trees.len());
            let pool = RpcPool::<SolanaRpcConnection>::new(config.clone()).await;
            let rollover_state = Arc::new(RolloverState::new());
            run_nullify(config, pool, rollover_state, state_trees).await;
        }
        Some(Commands::NullifyAddresses) => {
            let address_trees: Vec<_> = trees
                .iter()
                .filter(|&t| matches!(t.tree_type, TreeType::Address))
                .cloned()
                .collect();
            let pool = RpcPool::<SolanaRpcConnection>::new(config.clone()).await;
            let rollover_state = Arc::new(RolloverState::new());
            run_nullify(config, pool, rollover_state, address_trees).await;
        }
        Some(Commands::Nullify) => {
            let pool = RpcPool::<SolanaRpcConnection>::new(config.clone()).await;
            let rollover_state = Arc::new(RolloverState::new());
            run_nullify(config, pool, rollover_state, trees).await;
        }
        Some(Commands::TreeSync) => {
            let tree_data_list = fetch_trees(&config.external_services.rpc_url).await;
            serialize_tree_data(&tree_data_list).unwrap();
        }
        None => {
            return;
        }
        Some(Commands::StateQueueInfo) => {
            let rpc = SolanaRpcConnection::new(config.external_services.rpc_url.to_string(), None);
            let rpc = Arc::new(tokio::sync::Mutex::new(rpc));

            let state_trees: Vec<_> = trees
                .iter()
                .filter(|&t| matches!(t.tree_type, TreeType::State))
                .cloned()
                .collect();

            for tree_data in state_trees {
                let queue_length =
                    get_state_queue_length::<SolanaRpcConnection>(rpc.clone(), tree_data).await;
                println!(
                    "State queue {} length: {}",
                    tree_data.queue_pubkey, queue_length
                );
            }
        }

        Some(Commands::AddressQueueInfo) => {
            let rpc = SolanaRpcConnection::new(config.external_services.rpc_url.to_string(), None);
            let rpc = Arc::new(tokio::sync::Mutex::new(rpc));

            let address_trees: Vec<_> = trees
                .iter()
                .filter(|&t| matches!(t.tree_type, TreeType::Address))
                .cloned()
                .collect();

            for tree_data in address_trees {
                let queue_length =
                    get_address_queue_length::<SolanaRpcConnection>(rpc.clone(), tree_data).await;
                println!(
                    "Address queue {} length: {}",
                    tree_data.queue_pubkey, queue_length
                );
            }
        }
        Some(Commands::Airdrop) => {
            let mut rpc =
                SolanaRpcConnection::new(config.external_services.rpc_url.to_string(), None);
            rpc.airdrop_lamports(&rpc.payer.pubkey(), LAMPORTS_PER_SOL * 1000)
                .await
                .unwrap();
        }
    }
}

async fn run_nullify(
    config: Arc<ForesterConfig>,
    pool: RpcPool<SolanaRpcConnection>,
    rollover_state: Arc<RolloverState>,
    trees: Vec<TreeData>,
) {
    if trees.is_empty() {
        warn!("No trees found to nullify");
        return;
    }
    let mut handles = Vec::new();
    for tree_data in trees {
        let config_clone = config.clone();
        let pool_clone = pool.clone();
        let rollover_state = rollover_state.clone();
        let handle = tokio::spawn(async move {
            match tree_data.tree_type {
                TreeType::State => {
                    run_nullify_state(
                        config_clone.clone(),
                        pool_clone.clone(),
                        tree_data,
                        rollover_state.clone(),
                    )
                    .await
                }
                TreeType::Address => {
                    run_nullify_addresses(
                        config_clone.clone(),
                        pool_clone.clone(),
                        tree_data,
                        rollover_state.clone(),
                    )
                    .await
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all nullification processes to complete
    for handle in handles {
        match handle.await {
            Ok(result) => {
                info!("Nullification process completed: {:?}", result);
            }
            Err(e) => {
                error!("A nullification task panicked: {:?}", e);
            }
        }
    }

    debug!("All nullification processes completed");
}

async fn run_subscribe(
    config: Arc<ForesterConfig>,
    pool: RpcPool<SolanaRpcConnection>,
    rollover_state: Arc<RolloverState>,
    trees: Vec<TreeData>,
) {
    let mut handles = Vec::new();
    for tree_data in trees {
        let config_clone = config.clone();
        let pool_clone = pool.clone();
        let rollover_state = rollover_state.clone();
        let handle = tokio::spawn(async move {
            match tree_data.tree_type {
                TreeType::State => {
                    run_subscribe_state(
                        config_clone.clone(),
                        pool_clone.clone(),
                        tree_data,
                        rollover_state.clone(),
                    )
                    .await
                }
                TreeType::Address => {
                    run_subscribe_addresses(
                        config_clone.clone(),
                        pool_clone.clone(),
                        tree_data,
                        rollover_state.clone(),
                    )
                    .await
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all nullification processes to complete
    for handle in handles {
        if let Err(e) = handle.await {
            error!("A nullification process encountered an error: {:?}", e);
        }
    }

    debug!("All nullification processes completed");
}

async fn run_subscribe_state<R: RpcConnection>(
    config: Arc<ForesterConfig>,
    rpc_pool: RpcPool<R>,
    tree_data: TreeData,
    rollover_state: Arc<RolloverState>,
) {
    let indexer_rpc = R::new(config.external_services.rpc_url.to_string(), None);
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
        config.external_services.photon_api_key.clone(),
        indexer_rpc,
    )));

    subscribe_state(config.clone(), rpc_pool, indexer, tree_data, rollover_state).await;
}

async fn run_subscribe_addresses<R: RpcConnection>(
    config: Arc<ForesterConfig>,
    rpc_pool: RpcPool<R>,
    tree_data: TreeData,
    rollover_state: Arc<RolloverState>,
) {
    let indexer_rpc = R::new(config.external_services.rpc_url.to_string(), None);
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
        config.external_services.photon_api_key.clone(),
        indexer_rpc,
    )));

    subscribe_addresses(config.clone(), rpc_pool, indexer, tree_data, rollover_state).await;
}

async fn run_nullify_addresses<R: RpcConnection>(
    config: Arc<ForesterConfig>,
    rpc_pool: RpcPool<R>,
    tree_data: TreeData,
    rollover_state: Arc<RolloverState>,
) {
    let indexer_rpc = R::new(config.external_services.rpc_url.to_string(), None);
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
        config.external_services.photon_api_key.clone(),
        indexer_rpc,
    )));

    nullify_addresses(config.clone(), rpc_pool, indexer, tree_data, rollover_state).await;
}

async fn run_nullify_state<R: RpcConnection>(
    config: Arc<ForesterConfig>,
    rpc_pool: RpcPool<R>,
    tree_data: TreeData,
    rollover_state: Arc<RolloverState>,
) {
    let indexer_rpc = R::new(config.external_services.rpc_url.to_string(), None);
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
        config.external_services.photon_api_key.clone(),
        indexer_rpc,
    )));
    nullify_state(config.clone(), rpc_pool, indexer, tree_data, rollover_state).await;
}
