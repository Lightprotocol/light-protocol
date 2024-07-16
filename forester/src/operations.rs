use crate::config::ForesterConfig;
use crate::errors::ForesterError;
use crate::nullifier::address::setup_address_pipeline;
use crate::nullifier::state::setup_state_pipeline;
use crate::nullifier::{ForesterQueueAccount, ForesterQueueAccountData, QueueData};
use crate::rollover::{
    is_tree_ready_for_rollover, rollover_address_merkle_tree, rollover_state_merkle_tree,
    RolloverState,
};
use crate::tree_sync::TreeData;
use crate::{RpcPool, TreeType};
use account_compression::initialize_address_merkle_tree::Pubkey;
use account_compression::{AddressMerkleTreeAccount, QueueAccount};
use light_hash_set::HashSet;
use light_hasher::Poseidon;
use light_test_utils::get_indexed_merkle_tree;
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::{debug, info, warn};
use solana_client::pubsub_client::PubsubClient;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use std::mem;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;

pub async fn subscribe_state<I: Indexer<R>, R: RpcConnection>(
    config: Arc<ForesterConfig>,
    rpc_pool: RpcPool<R>,
    indexer: Arc<Mutex<I>>,
    tree_data: TreeData,
    rollover_state: Arc<RolloverState>,
) {
    debug!(
        "Subscribe to state tree changes. Queue: {}. Merkle tree: {}",
        tree_data.tree_pubkey, tree_data.queue_pubkey
    );
    loop {
        let (_account_subscription_client, account_subscription_receiver) =
            match PubsubClient::account_subscribe(
                &config.external_services.ws_rpc_url,
                &tree_data.queue_pubkey.clone(),
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
            let rpc_pool = rpc_pool.clone();
            match account_subscription_receiver.recv() {
                Ok(_) => {
                    debug!("nullify request received");
                    nullify_state(
                        Arc::clone(&config),
                        rpc_pool,
                        indexer.clone(),
                        tree_data,
                        rollover_state.clone(),
                    )
                    .await;
                }
                Err(e) => {
                    warn!("account subscription error: {:?}", e);
                    break;
                }
            }
        }
    }
}

pub async fn subscribe_addresses<I: Indexer<R>, R: RpcConnection>(
    config: Arc<ForesterConfig>,
    rpc_pool: RpcPool<R>,
    indexer: Arc<Mutex<I>>,
    tree_data: TreeData,
    rollover_state: Arc<RolloverState>,
) {
    debug!(
        "Subscribe to address tree changes. Queue: {}. Merkle tree: {}",
        tree_data.queue_pubkey, tree_data.tree_pubkey
    );
    loop {
        let (_account_subscription_client, account_subscription_receiver) =
            match PubsubClient::account_subscribe(
                &config.external_services.ws_rpc_url,
                &tree_data.queue_pubkey,
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
            let rpc_pool = rpc_pool.clone();
            match account_subscription_receiver.recv() {
                Ok(_) => {
                    debug!("nullify request received");
                    nullify_addresses(
                        Arc::clone(&config),
                        rpc_pool,
                        indexer.clone(),
                        tree_data,
                        rollover_state.clone(),
                    )
                    .await;
                }
                Err(e) => {
                    warn!("account subscription error: {:?}", e);
                    break;
                }
            }
        }
    }
}

pub async fn nullify_state<I: Indexer<R>, R: RpcConnection>(
    config: Arc<ForesterConfig>,
    rpc_pool: RpcPool<R>,
    indexer: Arc<Mutex<I>>,
    tree_data: TreeData,
    rollover_state: Arc<RolloverState>,
) {
    debug!(
        "Run state tree nullifier. Queue: {}. Merkle tree: {}",
        tree_data.queue_pubkey, tree_data.tree_pubkey
    );

    let (input_tx, mut completion_rx) = setup_state_pipeline(
        indexer.clone(),
        rpc_pool.clone(),
        config.clone(),
        tree_data,
        rollover_state.clone(),
    )
    .await;
    let result = completion_rx.recv().await;
    drop(input_tx);

    match result {
        Some(()) => {
            debug!("State nullifier completed successfully");
            let is_ready = is_tree_ready_for_rollover(
                &rpc_pool.get_connection().await,
                tree_data.tree_pubkey,
                TreeType::State,
            )
            .await
            .unwrap();
            if is_ready {
                debug!("State tree is ready for rollover");
                if rollover_state.try_start_rollover() {
                    match rollover_state_merkle_tree(&config, &rpc_pool, &indexer, &tree_data).await
                    {
                        Ok(_) => debug!("State tree rollover completed successfully"),
                        Err(e) => warn!("State tree rollover failed: {:?}", e),
                    }
                    rollover_state.end_rollover();
                } else {
                    debug!("Rollover already in progress, skipping");
                }
            }
        }
        None => {
            warn!("State nullifier stopped unexpectedly");
        }
    }
    // Optional: Add a small delay to allow the StreamProcessor to shut down gracefully
    tokio::time::sleep(Duration::from_millis(100)).await;
}

pub async fn nullify_addresses<I: Indexer<R>, R: RpcConnection>(
    config: Arc<ForesterConfig>,
    rpc_pool: RpcPool<R>,
    indexer: Arc<Mutex<I>>,
    tree_data: TreeData,
    rollover_state: Arc<RolloverState>,
) {
    debug!(
        "Run address tree nullifier. Queue: {}. Merkle tree: {}",
        tree_data.queue_pubkey, tree_data.tree_pubkey
    );

    let (input_tx, mut completion_rx) = setup_address_pipeline(
        indexer.clone(),
        rpc_pool.clone(),
        config.clone(),
        tree_data,
        rollover_state.clone(),
    )
    .await;
    let result = completion_rx.recv().await;
    drop(input_tx);

    match result {
        Some(()) => {
            info!("Address nullifier completed successfully");
            if is_tree_ready_for_rollover(
                &rpc_pool.get_connection().await,
                tree_data.tree_pubkey,
                TreeType::Address,
            )
            .await
            .unwrap()
            {
                info!("Address tree is ready for rollover");
                if rollover_state.try_start_rollover() {
                    match rollover_address_merkle_tree(&config, &rpc_pool, &indexer, &tree_data)
                        .await
                    {
                        Ok(_) => info!("Address tree rollover completed successfully"),
                        Err(e) => warn!("Address tree rollover failed: {:?}", e),
                    }
                    rollover_state.end_rollover();
                } else {
                    info!("Rollover already in progress, skipping");
                }
            }
        }
        None => {
            warn!("Address nullifier stopped unexpectedly");
        }
    }
    // Optional: Add a small delay to allow the AddressProcessor to shut down gracefully
    tokio::time::sleep(Duration::from_millis(100)).await;
}

pub async fn fetch_state_queue_data<R: RpcConnection>(
    rpc: Arc<Mutex<R>>,
    tree_data: TreeData,
) -> Result<QueueData, ForesterError> {
    debug!("Fetching state queue data");
    let mut rpc = rpc.lock().await;
    let mut nullifier_queue_account = rpc
        .get_account(tree_data.queue_pubkey)
        .await
        .map_err(|e| {
            warn!("Error fetching nullifier queue account: {:?}", e);
            ForesterError::Custom("Error fetching nullifier queue account".to_string())
        })?
        .unwrap();

    let nullifier_queue: HashSet = unsafe {
        HashSet::from_bytes_copy(
            &mut nullifier_queue_account.data[8 + mem::size_of::<QueueAccount>()..],
        )?
    };
    let mut queue_data = QueueData::new();

    for i in 0..nullifier_queue.capacity {
        let bucket = nullifier_queue.get_bucket(i).unwrap();
        if let Some(bucket) = bucket {
            if bucket.sequence_number.is_none() {
                let account = ForesterQueueAccount {
                    hash: bucket.value_bytes(),
                    index: i,
                };
                let account_data = ForesterQueueAccountData {
                    account,
                    proof: Vec::new(), // This will be filled in during FetchProofs stage
                    leaf_index: 0,     // This will be filled in during FetchProofs stage
                    root_seq: 0,       // This will be filled in during FetchProofs stage
                };
                queue_data.data.push(account_data);
            }
        }
    }
    debug!(
        "Fetched {} accounts from state queue",
        queue_data.data.len()
    );
    Ok(queue_data)
}

pub async fn fetch_address_queue_data<R: RpcConnection>(
    rpc: Arc<Mutex<R>>,
    tree_data: TreeData,
) -> Result<QueueData, ForesterError> {
    let mut rpc = rpc.lock().await;
    let mut account = rpc.get_account(tree_data.queue_pubkey).await?.unwrap();
    let address_queue: HashSet = unsafe {
        HashSet::from_bytes_copy(&mut account.data[8 + mem::size_of::<QueueAccount>()..])?
    };

    let mut queue_data = QueueData::new();

    for i in 0..address_queue.capacity {
        let bucket = address_queue.get_bucket(i).unwrap();
        if let Some(bucket) = bucket {
            if bucket.sequence_number.is_none() {
                queue_data.accounts.push(ForesterQueueAccount {
                    hash: bucket.value_bytes(),
                    index: i,
                });
            }
        }
    }
    Ok(queue_data)
}

#[allow(dead_code)]
pub async fn get_address_account_changelog_indices<R: RpcConnection>(
    merkle_tree_pubkey: &Pubkey,
    client: &mut R,
) -> Result<(usize, usize), ForesterError> {
    let merkle_tree =
        get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
            client,
            *merkle_tree_pubkey,
        )
        .await;
    let changelog_index = merkle_tree.changelog_index();
    let indexed_changelog_index = merkle_tree.indexed_changelog_index();
    Ok((changelog_index, indexed_changelog_index))
}
