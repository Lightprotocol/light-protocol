use crate::constants::{SERVER_URL, WS_SERVER_URL};
use solana_client::pubsub_client::PubsubClient;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use super::nullify;

pub async fn subscribe_nullify(
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    payer_keypair: Keypair,
) {
    let keypair = Arc::new(payer_keypair);

    loop {
        let (_account_subscription_client, account_subscription_receiver) =
            match PubsubClient::account_subscribe(
                WS_SERVER_URL,
                nullifier_queue_pubkey,
                Some(RpcAccountInfoConfig {
                    encoding: None,
                    data_slice: None,
                    commitment: Some(CommitmentConfig::confirmed()),
                    min_context_slot: None,
                }),
            ) {
                Ok((client, receiver)) => (client, receiver),
                Err(e) => {
                    println!("account subscription error: {:?}", e);
                    println!("retrying in 10 seconds...");
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
            };
        loop {
            match account_subscription_receiver.recv() {
                Ok(_) => {
                    println!("nullify request received");
                    let time = std::time::Instant::now();
                    match nullify(
                        nullifier_queue_pubkey,
                        merkle_tree_pubkey,
                        keypair.clone(),
                        SERVER_URL.to_string(),
                    )
                        .await
                    {
                        Ok(_) => {
                            println!("Nullify completed");
                            println!("Time elapsed: {:?}", time.elapsed());
                        }
                        Err(e) => {
                            println!("Error: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("account subscription error: {:?}", e);
                    break;
                }
            }
        }
    }
}