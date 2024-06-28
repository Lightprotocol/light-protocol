use anchor_lang::prelude::Pubkey;
use clap::Parser;
use config::Config;
use env_logger::Env;
use forester::cli::{Cli, Commands};
use forester::external_services_config::ExternalServicesConfig;
use forester::nqmt::reindex_and_store;
use forester::settings::SettingsKey;
use forester::{init_rpc, nullify_addresses, nullify_state, subscribe_state, ForesterConfig};
use light_registry::sdk::get_group_pda;
use light_test_utils::indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts, TestIndexer};
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::{GROUP_PDA_SEED_TEST_KEYPAIR, SIGNATURE_CPI_TEST_KEYPAIR};
use log::{error, info};
use serde_json::Result;
use solana_sdk::signature::{Keypair, Signer};
use std::env;
use std::str::FromStr;
use std::sync::Arc;

fn locate_config_file() -> String {
    let file_name = "forester.toml";

    let exe_path = env::current_exe().unwrap();
    let exe_dir = exe_path.parent().unwrap();
    let config_path = exe_dir.join(file_name);
    if config_path.exists() {
        return config_path.to_str().unwrap().to_string();
    }

    file_name.to_string()
}

fn convert(json: &str) -> Result<Vec<u8>> {
    serde_json::from_str(json)
}

fn init_config() -> ForesterConfig {
    let config_path = locate_config_file();

    let settings = Config::builder()
        .add_source(config::File::with_name(&config_path))
        .add_source(config::Environment::with_prefix("FORESTER"))
        .build()
        .unwrap();

    let state_merkle_tree_pubkey = settings
        .get_string(&SettingsKey::StateMerkleTreePubkey.to_string())
        .unwrap();
    let nullifier_queue_pubkey = settings
        .get_string(&SettingsKey::NullifierQueuePubkey.to_string())
        .unwrap();
    let address_merkle_tree_pubkey = settings
        .get_string(&SettingsKey::AddressMerkleTreePubkey.to_string())
        .unwrap();
    let address_merkle_tree_queue_pubkey = settings
        .get_string(&SettingsKey::AddressMerkleTreeQueuePubkey.to_string())
        .unwrap();
    let registry_pubkey = settings
        .get_string(&SettingsKey::RegistryPubkey.to_string())
        .unwrap();
    let payer = settings
        .get_string(&SettingsKey::Payer.to_string())
        .unwrap();
    let payer: Vec<u8> = convert(&payer).unwrap();

    ForesterConfig {
        external_services: ExternalServicesConfig::local(),
        nullifier_queue_pubkey: Pubkey::from_str(&nullifier_queue_pubkey).unwrap(),
        state_merkle_tree_pubkey: Pubkey::from_str(&state_merkle_tree_pubkey).unwrap(),
        address_merkle_tree_pubkey: Pubkey::from_str(&address_merkle_tree_pubkey).unwrap(),
        address_merkle_tree_queue_pubkey: Pubkey::from_str(&address_merkle_tree_queue_pubkey)
            .unwrap(),
        registry_pubkey: Pubkey::from_str(&registry_pubkey).unwrap(),
        payer_keypair: Keypair::from_bytes(&payer).unwrap(),
        concurrency_limit: 10,
        batch_size: 10,
        max_retries: 5,
        max_concurrent_batches: 1,
    }
}
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let config: Arc<ForesterConfig> = Arc::new(init_config());
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Subscribe) => {
            info!(
                "Subscribe to nullify compressed accounts for indexed array: {} and merkle tree: {}",
                config.nullifier_queue_pubkey, config.state_merkle_tree_pubkey
            );
            subscribe_state(config.clone()).await;
        }
        Some(Commands::NullifyState) => {
            nullify_state(config).await;
        }
        Some(Commands::NullifyAddresses) => {
            run_nullify_addresses(config).await;
            /*
            let rpc = init_rpc(&config).await;
            let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
                config.external_services.indexer_url.to_string(),
            )));
            */
        }
        Some(Commands::Nullify) => {
            let state_nullifier = tokio::spawn(nullify_state(config.clone()));
            let address_nullifier = tokio::spawn(run_nullify_addresses(config));

            // Wait for both nullifiers to complete
            let (state_result, address_result) = tokio::join!(state_nullifier, address_nullifier);

            if let Err(e) = state_result {
                error!("State nullifier encountered an error: {:?}", e);
            }

            if let Err(e) = address_result {
                error!("Address nullifier encountered an error: {:?}", e);
            }

            info!("All nullification processes completed");
        }
        Some(Commands::Index) => {
            info!("Reindex merkle tree & nullifier queue accounts");
            info!(
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

async fn run_nullify_addresses(config: Arc<ForesterConfig>) {
    let rpc = init_rpc(&config).await;
    let rpc = Arc::new(tokio::sync::Mutex::new(rpc));

    let cpi_context_account_keypair = Keypair::from_bytes(&SIGNATURE_CPI_TEST_KEYPAIR).unwrap();
    let group_seed_keypair = Keypair::from_bytes(&GROUP_PDA_SEED_TEST_KEYPAIR).unwrap();
    let group_pda = get_group_pda(group_seed_keypair.pubkey());

    let indexer: TestIndexer<200, SolanaRpcConnection> = TestIndexer::new(
        vec![StateMerkleTreeAccounts {
            merkle_tree: config.state_merkle_tree_pubkey,
            nullifier_queue: config.nullifier_queue_pubkey,
            cpi_context: cpi_context_account_keypair.pubkey(),
        }],
        vec![AddressMerkleTreeAccounts {
            merkle_tree: config.address_merkle_tree_pubkey,
            queue: config.address_merkle_tree_queue_pubkey,
        }],
        config.payer_keypair.insecure_clone(),
        group_pda,
        true,
        true,
    )
    .await;
    let indexer = Arc::new(tokio::sync::Mutex::new(indexer));
    nullify_addresses(config.clone(), rpc, indexer).await;
}
