use crate::config::ForesterConfig;
use crate::indexer::PhotonIndexer;
use crate::nullifier::address::setup_address_pipeline;
use crate::nullifier::state::setup_state_pipeline;
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use log::{debug, info, warn};
use solana_client::pubsub_client::PubsubClient;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_sdk::signature::Signer;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

pub async fn subscribe_state(config: Arc<ForesterConfig>) {
    debug!(
        "Subscribe to state tree changes. Queue: {}. Merkle tree: {}",
        config.nullifier_queue_pubkey, config.state_merkle_tree_pubkey
    );
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
                    warn!("account subscription error: {:?}", e);
                    warn!("retrying in 500ms...");
                    sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };
        loop {
            match account_subscription_receiver.recv() {
                Ok(_) => {
                    debug!("nullify request received");
                    nullify_state(Arc::clone(&config)).await;
                }
                Err(e) => {
                    warn!("account subscription error: {:?}", e);
                    break;
                }
            }
        }
    }
}

pub async fn nullify_state(config: Arc<ForesterConfig>) {
    debug!(
        "Run state tree nullifier. Queue: {}. Merkle tree: {}",
        config.nullifier_queue_pubkey, config.state_merkle_tree_pubkey
    );
    let rpc = init_rpc(&config).await;
    let indexer = Arc::new(tokio::sync::Mutex::new(PhotonIndexer::new(
        config.external_services.indexer_url.to_string(),
    )));
    let rpc = Arc::new(tokio::sync::Mutex::new(rpc));

    let (input_tx, mut completion_rx) = setup_state_pipeline(indexer, rpc, config).await;
    let result = completion_rx.recv().await;
    drop(input_tx);

    match result {
        Some(()) => {
            debug!("State nullifier completed successfully");
        }
        None => {
            warn!("State nullifier stopped unexpectedly");
        }
    }
    // Optional: Add a small delay to allow the StreamProcessor to shut down gracefully
    tokio::time::sleep(Duration::from_millis(100)).await;
}

pub async fn nullify_addresses<I: Indexer, R: RpcConnection>(
    config: Arc<ForesterConfig>,
    rpc: Arc<tokio::sync::Mutex<R>>,
    indexer: Arc<tokio::sync::Mutex<I>>,
) {
    debug!(
        "Run address tree nullifier. Queue: {}. Merkle tree: {}",
        config.address_merkle_tree_queue_pubkey, config.address_merkle_tree_pubkey
    );

    let (input_tx, mut completion_rx) = setup_address_pipeline(indexer, rpc, config).await;
    let result = completion_rx.recv().await;
    drop(input_tx);

    match result {
        Some(()) => {
            info!("Address nullifier completed successfully");
        }
        None => {
            warn!("Address nullifier stopped unexpectedly");
        }
    }
    // Optional: Add a small delay to allow the AddressProcessor to shut down gracefully
    tokio::time::sleep(Duration::from_millis(100)).await;
}

pub async fn init_rpc(config: &Arc<ForesterConfig>) -> SolanaRpcConnection {
    let mut rpc = SolanaRpcConnection::new(
        config.external_services.rpc_url.clone(),
        Some(CommitmentConfig {
            commitment: CommitmentLevel::Confirmed,
        }),
    );

    rpc.airdrop_lamports(&config.payer_keypair.pubkey(), 10_000_000_000)
        .await
        .unwrap();

    rpc
}
