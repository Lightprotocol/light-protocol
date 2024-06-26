use super::Config;
use crate::indexer::PhotonIndexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::{info, warn};
use solana_client::pubsub_client::PubsubClient;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub async fn subscribe_nullify<R: RpcConnection + Clone + Send + Sync + 'static>(
    config: &Config,
    rpc: R,
) {
    let _indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
    )));
    let _rpc = Arc::new(tokio::sync::Mutex::new(rpc));
    let config = Arc::new(config.clone());

    loop {
        let (_account_subscription_client, account_subscription_receiver) =
            match PubsubClient::account_subscribe(
                &config.external_services.ws_rpc_url,
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
                    // TODO: update this to use the new pipeline
                    // let time = std::time::Instant::now();
                    // let indexer_clone = Arc::clone(&indexer);
                    // let rpc_clone = Arc::clone(&rpc);
                    // let config = Arc::clone(&config);
                    // match nullify(indexer_clone, rpc_clone, config).await {
                    //     Ok(_) => {
                    //         info!("Nullify completed");
                    //         info!("Time elapsed: {:?}", time.elapsed());
                    //     }
                    //     Err(e) => {
                    //         warn!("Error: {:?}", e);
                    //     }
                    // }
                }
                Err(e) => {
                    warn!("account subscription error: {:?}", e);
                    break;
                }
            }
        }
    }
}
