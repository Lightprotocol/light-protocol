use anchor_lang::prelude::Pubkey;
use clap::Parser;
use config::Config;
use env_logger::Env;
use forester::nqmt::reindex_and_store;
use log::info;
use solana_sdk::signature::{Keypair, Signer};
use std::str::FromStr;

use forester::cli::{Cli, Commands};
use forester::constants::{INDEXER_URL, SERVER_URL};
use forester::indexer::PhotonIndexer;
use forester::nullifier::Config as ForesterConfig;
use forester::nullifier::{nullify, subscribe_nullify};
use forester::settings::SettingsKey;
use light_test_utils::rpc::SolanaRpcConnection;
use std::env;
use std::sync::Arc;
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use light_test_utils::rpc::rpc_connection::RpcConnection;

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
    let payer = settings.get_array(&SettingsKey::Payer.to_string()).unwrap();
    let payer: Vec<u8> = payer
        .iter()
        .map(|v| v.clone().into_uint().unwrap() as u8)
        .collect();

    ForesterConfig {
        server_url: SERVER_URL.to_string(),
        nullifier_queue_pubkey: Pubkey::from_str(&nullifier_queue_pubkey).unwrap(),
        state_merkle_tree_pubkey: Pubkey::from_str(&state_merkle_tree_pubkey).unwrap(),
        address_merkle_tree_pubkey: Pubkey::from_str(&address_merkle_tree_pubkey).unwrap(),
        address_merkle_tree_queue_pubkey: Pubkey::from_str(&address_merkle_tree_queue_pubkey)
            .unwrap(),
        registry_pubkey: Pubkey::from_str(&registry_pubkey).unwrap(),
        payer_keypair: Keypair::from_bytes(&payer).unwrap(),
        concurrency_limit: 1,
        batch_size: 1000,
        max_retries: 5,
        max_concurrent_batches: 5,
    }
}
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let config = init_config();

    let commitment_config = CommitmentConfig {
        commitment: CommitmentLevel::Confirmed,
    };

    let mut rpc = SolanaRpcConnection::new(Some(commitment_config));

    rpc.airdrop_lamports(
        &config.payer_keypair.pubkey(),
        10_000_000_000,
    ).await.unwrap();


    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Subscribe) => {
            info!(
                "Subscribe to nullify compressed accounts for indexed array: {} and merkle tree: {}",
                config.nullifier_queue_pubkey, config.state_merkle_tree_pubkey
            );
            subscribe_nullify(&config, rpc.clone()).await;
        }
        Some(Commands::Nullify) => {
            info!(
                "Nullify compressed accounts for nullifier queue: {} and merkle tree: {}",
                config.nullifier_queue_pubkey, config.state_merkle_tree_pubkey
            );

            let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(INDEXER_URL.to_string())));
            let rpc = Arc::new(tokio::sync::Mutex::new(rpc));

            let result = nullify(indexer, rpc, &config).await;
            info!("Nullification result: {:?}", result);
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
