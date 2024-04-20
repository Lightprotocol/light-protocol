use std::str::FromStr;
use clap::Parser;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::read_keypair_file;
use account_compression::Pubkey;
use crank::cli::{Cli, Commands};
use crank::constants::SERVER_URL;
use crank::nullifier::{nullify_compressed_accounts, subscribe_nullify};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let client = RpcClient::new(SERVER_URL);

    match &cli.command {
        Some(Commands::NullifyCompressedAccounts {
            nullifier_queue_pubkey,
            merkle_tree_pubkey,
        }) => {
            println!(
                "Nullify compressed accounts for nullifier queue: {} and merkle tree: {}",
                nullifier_queue_pubkey, merkle_tree_pubkey
            );
            let nullifier_queue_pubkey = Pubkey::from_str(nullifier_queue_pubkey).unwrap();
            let merkle_tree_pubkey = Pubkey::from_str(merkle_tree_pubkey).unwrap();
            let payer_keypair = read_keypair_file("keypair.json").unwrap();


           nullify_compressed_accounts(&nullifier_queue_pubkey, &merkle_tree_pubkey, &payer_keypair, &client);
        }
        Some(Commands::SubscribeNullify {
            nullifier_queue_pubkey,
            merkle_tree_pubkey,
        }) => {
            println!(
                "Subscribe to nullify compressed accounts for indexed array: {} and merkle tree: {}",
                nullifier_queue_pubkey, merkle_tree_pubkey
            );
            let nullifier_queue_pubkey = Pubkey::from_str(nullifier_queue_pubkey).unwrap();
            let merkle_tree_pubkey = Pubkey::from_str(merkle_tree_pubkey).unwrap();
            let payer_keypair = read_keypair_file("keypair.json").unwrap();
            subscribe_nullify(&nullifier_queue_pubkey, &merkle_tree_pubkey, &payer_keypair, &client);
        }
        None => {
            return;
        }
    }
}