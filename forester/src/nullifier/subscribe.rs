use super::{nullify, Config};
use crate::constants::{INDEXER_URL, WS_SERVER_URL};
use crate::indexer::PhotonIndexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::{info, warn};
use solana_client::pubsub_client::PubsubClient;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use tokio::time::{sleep, Duration};

pub async fn subscribe_nullify<R: RpcConnection>(config: &Config, rpc: &mut R) {
    let mut indexer = PhotonIndexer::new(INDEXER_URL.to_string());
    loop {
        let (_account_subscription_client, account_subscription_receiver) =
            match PubsubClient::account_subscribe(
                WS_SERVER_URL,
                &config.nullifier_queue_pubkey,
                Some(RpcAccountInfoConfig {
                    encoding: None,
                    data_slice: None,
                    commitment: Some(CommitmentConfig::confirmed()),
                    min_context_slot: None,
                }),
            ) {
                Ok((client, receiver)) => (client, receiver),
                Err(e) => {
                    info!("account subscription error: {:?}", e);
                    info!("retrying in 500ms...");
                    sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };
        loop {
            match account_subscription_receiver.recv() {
                Ok(_) => {
                    info!("nullify request received");
                    let time = std::time::Instant::now();
                    match nullify(&mut indexer, rpc, config).await {
                        Ok(_) => {
                            info!("Nullify completed");
                            info!("Time elapsed: {:?}", time.elapsed());
                        }
                        Err(e) => {
                            warn!("Error: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("account subscription error: {:?}", e);
                    break;
                }
            }
        }
    }
}
