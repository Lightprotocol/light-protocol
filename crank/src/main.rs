use std::sync::Arc;

use clap::Parser;
use config::Config;
use crank::nqmt::reindex_and_store;
use solana_sdk::signature::{Keypair, Signer};

use crank::cli::{Cli, Commands};
use crank::constants::SERVER_URL;
use crank::nullifier::{nullify, subscribe_nullify};
use crank::utils::request_airdrop;

#[tokio::main]
async fn main() {
    println!("Run crank");
    let settings = Config::builder()
        // add in `./crank.toml`
        .add_source(config::File::with_name("crank"))
        // add in settings from the environment (with a prefix of CRANK)
        // Eg.. `CRANK_DEBUG=1 ./target/app` would set the `debug` key
        .add_source(config::Environment::with_prefix("CRANK"))
        .build()
        .unwrap();

    let merkle_tree = settings.get_string("merkle_tree").unwrap();
    let nullifier_queue = settings.get_string("nullifier_queue").unwrap();
    let payer = settings.get_string("payer").unwrap();

    let merkle_tree_keypair = Keypair::from_base58_string(&merkle_tree);
    let nullifier_queue_keypair = Keypair::from_base58_string(&nullifier_queue);
    let payer_keypair = Keypair::from_base58_string(&payer);

    request_airdrop(&payer_keypair.pubkey());

    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Subscribe) => {
            println!(
                "Subscribe to nullify compressed accounts for indexed array: {} and merkle tree: {}",
                nullifier_queue_keypair.pubkey(), merkle_tree_keypair.pubkey()
            );
            subscribe_nullify(
                &nullifier_queue_keypair.pubkey(),
                &merkle_tree_keypair.pubkey(),
                payer_keypair,
            )
            .await;
        }
        Some(Commands::Nullify) => {
            println!(
                "Nullify compressed accounts for nullifier queue: {} and merkle tree: {}",
                nullifier_queue_keypair.pubkey(),
                merkle_tree_keypair.pubkey()
            );
            let nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();
            let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
            let payer = Arc::new(payer_keypair);
            let _ = nullify(
                &nullifier_queue_pubkey,
                &merkle_tree_pubkey,
                payer,
                SERVER_URL.to_string(),
            )
            .await;
        }
        Some(Commands::Index) => {
            println!("Reindex merkle tree & nullifier queue accounts");
            println!("Initial merkle tree account: {}", merkle_tree_keypair.pubkey());
            let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
            let _ = reindex_and_store(
                &merkle_tree_pubkey,
                SERVER_URL,
            );
        }
        None => {
            return;
        }
    }
}
