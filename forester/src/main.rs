use clap::Parser;
use env_logger::Env;
use forester::cli::{Cli, Commands};
use forester::indexer::PhotonIndexer;
use forester::{get_address_queue_length, get_state_queue_length, init_config, init_rpc, nullify_addresses, nullify_state, subscribe_addresses, subscribe_state, ForesterConfig, RpcPool, TreeType};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use log::{debug, error, info};
use std::sync::Arc;
use forester::rollover::RolloverState;
use forester::tree_sync::{serialize_tree_data, sync, TreeData};

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
            let pool = RpcPool::<SolanaRpcConnection>::new(config.clone()).await;
            let state_tree_data = TreeData::default_state();
            let address_tree_data = TreeData::default_address();
            let state_rollover_state = Arc::new(RolloverState::new());
            // let state_nullifier = tokio::spawn(run_subscribe_state(config.clone(), pool.clone(), state_tree_data, state_rollover_state));

            let address_rollover_state = Arc::new(RolloverState::new());
            // let address_nullifier = tokio::spawn(run_subscribe_addresses(config, pool.clone(), address_tree_data, address_rollover_state));

            // Wait for both nullifiers to complete
            // let (state_result, address_result) = tokio::join!(state_nullifier, address_nullifier);

            // if let Err(e) = state_result {
            //     error!("State nullifier encountered an error: {:?}", e);
            // }
            //
            // if let Err(e) = address_result {
            //     error!("Address nullifier encountered an error: {:?}", e);
            // }

            debug!("All nullification processes completed");
        }
        Some(Commands::NullifyState) => {
            let state_tree_data = TreeData::default_state();
            let pool = RpcPool::<SolanaRpcConnection>::new(config.clone()).await;
            let rollover_state = Arc::new(RolloverState::new());
            run_nullify_state(config, pool, state_tree_data, rollover_state).await;
        }
        Some(Commands::NullifyAddresses) => {
            let address_tree_data = TreeData::default_address();
            let pool = RpcPool::<SolanaRpcConnection>::new(config.clone()).await;
            let rollover_state = Arc::new(RolloverState::new());
            run_nullify_addresses(config, pool, address_tree_data, rollover_state).await;
        }
        Some(Commands::Nullify) => {
            let state_tree_data = TreeData::default_state();
            let address_tree_data = TreeData::default_address();
            let pool = RpcPool::<SolanaRpcConnection>::new(config.clone()).await;

            let state_rollover_state = Arc::new(RolloverState::new());
            let state_nullifier = tokio::spawn(run_nullify_state(config.clone(), pool.clone(), state_tree_data, state_rollover_state));

            // let address_rollover_state = Arc::new(RolloverState::new());
            // let address_nullifier = tokio::spawn(run_nullify_addresses(config, pool.clone(), address_tree_data, address_rollover_state));

            // Wait for both nullifiers to complete
            // let (state_result, address_result) = tokio::join!(state_nullifier, address_nullifier);

            // if let Err(e) = state_result {
            //     error!("State nullifier encountered an error: {:?}", e);
            // }
            //
            // if let Err(e) = address_result {
            //     error!("Address nullifier encountered an error: {:?}", e);
            // }

            debug!("All nullification processes completed");
        }
        Some(Commands::TreeSync) => {
            let synced_tree_data = sync(&config, &config.external_services.rpc_url).await.unwrap();
            info!("Synced tree data: {:?}", synced_tree_data);
            serialize_tree_data(&synced_tree_data).unwrap();
        }
        None => {
            return;
        }
        Some(Commands::StateQueueInfo) => {
            let rpc: SolanaRpcConnection = init_rpc(config.clone(), false).await;
            let rpc = Arc::new(tokio::sync::Mutex::new(rpc));
            let queue_length = get_state_queue_length::<SolanaRpcConnection>(rpc).await;
            println!("State queue length: {}", queue_length);
        }

        Some(Commands::AddressQueueInfo) => {
            let rpc: SolanaRpcConnection = init_rpc(config.clone(), false).await;
            let rpc = Arc::new(tokio::sync::Mutex::new(rpc));
            let queue_length = get_address_queue_length::<SolanaRpcConnection>(rpc).await;
            println!("Address queue length: {}", queue_length);
        }
        Some(Commands::Airdrop) => {
            init_rpc::<SolanaRpcConnection>(config.clone(), true).await;
        }
    }
}

async fn run_subscribe(config: Arc<ForesterConfig>, pool: RpcPool<SolanaRpcConnection>, rollover_state: Arc<RolloverState>,) {
    let tree_data_list = match sync(&config, &config.external_services.rpc_url).await {
        Ok(tree_data) => tree_data,
        Err(e) => {
            error!("Failed to reindex trees: {:?}", e);
            return;
        }
    };

    if let Err(e) = serialize_tree_data(&tree_data_list) {
        error!("Failed to serialize indexed trees: {:?}", e);
        return;
    }

    // for tree_data in tree_data_list {
    //     match tree_data.tree_type {
    //         TreeType::State => {
    //             run_subscribe_state(config.clone(), pool.clone(), tree_data, rollover_state.clone()).await;
    //         }
    //         TreeType::Address => {
    //             run_subscribe_addresses(config.clone(), pool.clone(), tree_data, rollover_state.clone()).await;
    //         }
    //     }
    // }

    // let mut handles = Vec::new();
    // for tree_data in tree_data_list {
    //     let config_clone = config.clone();
    //     let pool_clone = pool.clone();
    //     let rollover_state = rollover_state.clone();
        // let handle = tokio::spawn(async move {
        //     match tree_data.tree_type {
        //         TreeType::State => {
        //             run_subscribe_state(config_clone.clone(), pool_clone.clone(), tree_data, rollover_state.clone()).await
        //         }
        //         TreeType::Address => {
        //             run_subscribe_addresses(config_clone.clone(), pool_clone.clone(), tree_data, rollover_state.clone()).await
        //         }
        //     }
        // });
        // handles.push(handle);
    // }
    // Wait for all nullification processes to complete
    // for handle in handles {
    //     if let Err(e) = handle.await {
    //         error!("A nullification process encountered an error: {:?}", e);
    //     }
    // }


    debug!("All nullification processes completed");
}

async fn run_subscribe_state(config: Arc<ForesterConfig>, rpc_pool: RpcPool<SolanaRpcConnection>, tree_data: TreeData, rollover_state: Arc<RolloverState>) {
    let indexer_rpc = init_rpc(config.clone(), false).await;
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
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
    let indexer_rpc = init_rpc(config.clone(), false).await;
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
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
    let indexer_rpc = init_rpc(config.clone(), false).await;
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
        indexer_rpc,
    )));

    nullify_addresses(config.clone(), rpc_pool, indexer, tree_data, rollover_state).await;
}

async fn run_nullify_state<R: RpcConnection>(config: Arc<ForesterConfig>, rpc_pool: RpcPool<R>, tree_data: TreeData, rollover_state: Arc<RolloverState>,) {
    let indexer_rpc = init_rpc(config.clone(), false).await;
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
        indexer_rpc,
    )));

    nullify_state(config.clone(), rpc_pool, indexer, tree_data, rollover_state).await;
}
