use clap::Parser;
use config::Config;
use env_logger::Env;
use forester::nqmt::reindex_and_store;
use log::info;
use solana_sdk::signature::{Keypair, Signer};

use forester::cli::{Cli, Commands};
use forester::constants::{INDEXER_URL, SERVER_URL};
use forester::indexer::PhotonIndexer;
use forester::nullifier::Config as ForesterConfig;
use forester::nullifier::{nullify, subscribe_nullify};

fn init_config() -> ForesterConfig {
    let settings = Config::builder()
        .add_source(config::File::with_name("forester"))
        .add_source(config::Environment::with_prefix("FORESTER"))
        .build()
        .unwrap();

    let merkle_tree = settings.get_string("merkle_tree").unwrap();
    let nullifier_queue = settings.get_string("nullifier_queue").unwrap();
    let payer = settings.get_string("payer").unwrap();

    let merkle_tree_keypair = Keypair::from_base58_string(&merkle_tree);
    let nullifier_queue_keypair = Keypair::from_base58_string(&nullifier_queue);
    let payer_keypair = Keypair::from_base58_string(&payer);

    ForesterConfig {
        server_url: SERVER_URL.to_string(),
        nullifier_queue_pubkey: nullifier_queue_keypair.pubkey(),
        merkle_tree_pubkey: merkle_tree_keypair.pubkey(),
        payer_keypair: payer_keypair.insecure_clone(),
        concurrency_limit: 20,
        batch_size: 1000,
        max_retries: 5,
    }
}
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let config = init_config();

    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Subscribe) => {
            info!(
                "Subscribe to nullify compressed accounts for indexed array: {} and merkle tree: {}",
                config.nullifier_queue_pubkey, config.merkle_tree_pubkey
            );

            subscribe_nullify(&config).await;
        }
        Some(Commands::Nullify) => {
            info!(
                "Nullify compressed accounts for nullifier queue: {} and merkle tree: {}",
                config.nullifier_queue_pubkey, config.merkle_tree_pubkey
            );

            let indexer = PhotonIndexer::new(INDEXER_URL.to_string());
            let result = nullify(indexer, &config).await;
            info!("Nullification result: {:?}", result);
        }
        Some(Commands::Index) => {
            info!("Reindex merkle tree & nullifier queue accounts");
            info!("Initial merkle tree account: {}", config.merkle_tree_pubkey);
            let _ = reindex_and_store(&config);
        }
        None => {
            return;
        }
    }
}
