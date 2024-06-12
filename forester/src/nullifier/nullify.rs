use crate::errors::ForesterError;
use crate::nullifier::queue_data::Account;
use crate::nullifier::{Config, StateQueueData};
use account_compression::{QueueAccount, StateMerkleTreeAccount};
use light_hash_set::HashSet;
use light_hasher::Poseidon;
use light_registry::sdk::{create_nullify_instruction, CreateNullifyInstructionInputs};
use light_test_utils::get_concurrent_merkle_tree;
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::errors::RpcError;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use log::{info, warn};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use std::collections::HashMap;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;

pub async fn nullify<T: Indexer>(indexer: T, config: &Config) -> Result<(), ForesterError> {
    let arc_indexer = Arc::new(Mutex::new(indexer));
    let concurrency_limit = config.concurrency_limit;
    let semaphore = Arc::new(Semaphore::new(concurrency_limit));
    let successful_nullifications = Arc::new(Mutex::new(1));
    let cancellation_token = CancellationToken::new();
    let cancellation_token_clone = cancellation_token.clone();
    let _terminate_handle = {
        tokio::spawn(async move {
            if signal::ctrl_c().await.is_ok() {
                cancellation_token_clone.cancel();
                info!("Ctrl-C received, canceling tasks...");
            }
        })
    };

    loop {
        let queue_data = {
            let mut queue_data = fetch_queue_data(arc_indexer.clone(), config).await?;

            if cancellation_token.is_cancelled() {
                info!("Cancellation detected, exiting loop...");
                break;
            }
            if queue_data.is_none()
                || *successful_nullifications.lock().await % config.batch_size == 0
            {
                match fetch_queue_data(arc_indexer.clone(), config).await {
                    Ok(data) => {
                        queue_data = data;
                    }
                    Err(e) => match e {
                        ForesterError::NoProofsFound => {
                            warn!("No proofs found. Please check that nullifier queue is empty");
                            break;
                        }
                        _ => {
                            warn!("Error fetching queue data: {:?}", e);
                            continue;
                        }
                    },
                }
            }

            queue_data
        };

        if queue_data.is_none() {
            info!("No more accounts to nullify. Exiting...");
            cancellation_token.cancel();
            break;
        }

        let permit = Arc::clone(&semaphore).acquire_owned().await;
        let max_retries = config.max_retries;
        let mut queue_data = queue_data.unwrap();
        let account = queue_data.compressed_accounts_to_nullify.remove(0);
        if let Some((proof, leaf_index, _)) = queue_data
            .compressed_account_proofs
            .remove(&account.hash_string())
        {
            let client =
                RpcClient::new_with_commitment(&config.server_url, CommitmentConfig::confirmed());
            let successful_nullifications = Arc::clone(&successful_nullifications);
            let cancellation_token_clone = cancellation_token.clone();
            let arc_indexer_clone = Arc::clone(&arc_indexer);
            let config_clone = config.clone();
            let task = tokio::spawn(async move {
                let _permit = permit;
                let mut retries = 0;
                loop {
                    if cancellation_token_clone.is_cancelled() {
                        info!("Task cancelled for account {}", account.hash_string());
                        break;
                    }
                    let proof_clone = proof.clone();
                    info!("Nullifying account: {}", account.hash_string());
                    match nullify_compressed_account(
                        account,
                        queue_data.change_log_index,
                        proof_clone,
                        leaf_index,
                        &config_clone,
                        &client,
                        arc_indexer_clone.clone(),
                    )
                    .await
                    {
                        Ok(_) => {
                            let mut successful_nullifications =
                                successful_nullifications.lock().await;
                            *successful_nullifications += 1;
                            break;
                        }
                        Err(e) => {
                            if retries >= max_retries {
                                warn!(
                                    "Max retries reached for account {}: {:?}",
                                    account.hash_string(),
                                    e
                                );
                                break;
                            }
                            retries += 1;
                            warn!(
                                "Retrying account {} due to error: {:?}",
                                account.hash_string(),
                                e
                            );
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        }
                    }
                }
            });
            if let Err(e) = task.await {
                warn!("Task failed with error: {:?}", e);
                continue;
            }
        } else {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            continue;
        }
    }

    // TODO: should we use terminate_handle.await.unwrap() here?
    let successful_nullifications = successful_nullifications.lock().await;
    info!("Successful nullifications: {}", *successful_nullifications);

    Ok(())
}

async fn fetch_queue_data<T: Indexer>(
    indexer: Arc<Mutex<T>>,
    config: &Config,
) -> Result<Option<StateQueueData>, ForesterError> {
    let (change_log_index, sequence_number) = {
        let mut temporary_client = SolanaRpcConnection::new_with_url(
            &config.server_url,
            Some(CommitmentConfig::processed()),
        );
        get_changelog_index(&config.state_merkle_tree_pubkey, &mut temporary_client).await?
    };
    let compressed_accounts_to_nullify = {
        let temporary_client =
            RpcClient::new_with_commitment(&config.server_url, CommitmentConfig::processed());
        let queue = get_nullifier_queue(&config.nullifier_queue_pubkey, &temporary_client)?;
        info!(
            "Queue length: {}. Trimming to batch size of {}...",
            queue.len(),
            config.batch_size
        );
        queue
            .into_iter()
            .take(config.batch_size)
            .collect::<Vec<_>>()
    };

    if compressed_accounts_to_nullify.is_empty() {
        return Ok(None);
    }

    let compressed_account_list = compressed_accounts_to_nullify
        .iter()
        .map(|account| account.hash_string())
        .collect::<Vec<_>>();

    let indexer_guard = indexer.lock().await;
    let indexer = indexer_guard.deref();
    let proofs = indexer
        .get_multiple_compressed_account_proofs(compressed_account_list.clone())
        .await
        .map_err(|e| {
            warn!("Cannot get multiple proofs: {:#?}", e);
            ForesterError::NoProofsFound
        })?;
    let compressed_account_proofs: HashMap<String, (Vec<[u8; 32]>, u64, i64)> = proofs
        .into_iter()
        .map(|proof| {
            (
                proof.hash.clone(),
                (proof.proof.clone(), proof.leaf_index as u64, proof.root_seq),
            )
        })
        .collect();
    Ok(Some(StateQueueData {
        change_log_index,
        sequence_number,
        compressed_accounts_to_nullify,
        compressed_account_proofs,
    }))
}

#[allow(clippy::too_many_arguments)]
pub async fn nullify_compressed_account<T: Indexer>(
    account: Account,
    change_log_index: usize,
    proof: Vec<[u8; 32]>,
    leaf_index: u64,
    config: &Config,
    client: &RpcClient,
    indexer: Arc<Mutex<T>>,
) -> Result<(), ForesterError> {
    let ix = create_nullify_instruction(CreateNullifyInstructionInputs {
        nullifier_queue: config.nullifier_queue_pubkey,
        merkle_tree: config.state_merkle_tree_pubkey,
        change_log_indices: vec![change_log_index as u64],
        leaves_queue_indices: vec![account.index as u16],
        indices: vec![leaf_index],
        proofs: vec![proof],
        authority: config.payer_keypair.pubkey(),
    });
    let instructions = [
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
        ix,
    ];

    let latest_blockhash = client.get_latest_blockhash().map_err(RpcError::from)?;
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&config.payer_keypair.pubkey()),
        &[&config.payer_keypair],
        latest_blockhash,
    );

    let tx_config = RpcSendTransactionConfig {
        skip_preflight: true,
        preflight_commitment: Some(solana_sdk::commitment_config::CommitmentLevel::Confirmed),
        ..RpcSendTransactionConfig::default()
    };
    let signature = client
        .send_transaction_with_config(&transaction, tx_config)
        .map_err(RpcError::from)?;
    info!("Transaction: {:?}", signature);
    loop {
        let confirmed = client.confirm_transaction(&signature).unwrap();
        if confirmed {
            break;
        }
    }

    let mut guard = indexer.lock().await;
    let indexer = guard.deref_mut();
    indexer.account_nullified(config.state_merkle_tree_pubkey, &account.hash_string());
    Ok(())
}

pub fn get_nullifier_queue(
    nullifier_queue_pubkey: &Pubkey,
    client: &RpcClient,
) -> Result<Vec<Account>, ForesterError> {
    let mut nullifier_queue_account = client
        .get_account(nullifier_queue_pubkey)
        .map_err(RpcError::from)?;
    let nullifier_queue: HashSet = unsafe {
        HashSet::from_bytes_copy(
            &mut nullifier_queue_account.data[8 + mem::size_of::<QueueAccount>()..],
        )?
    };
    let mut compressed_accounts_to_nullify = Vec::new();
    for i in 0..nullifier_queue.capacity {
        let bucket = nullifier_queue.get_bucket(i).unwrap();
        if let Some(bucket) = bucket {
            if bucket.sequence_number.is_none() {
                compressed_accounts_to_nullify.push(Account {
                    hash: bucket.value_bytes(),
                    index: i,
                });
            }
        }
    }
    Ok(compressed_accounts_to_nullify)
}

pub async fn get_changelog_index<R: RpcConnection>(
    merkle_tree_pubkey: &Pubkey,
    client: &mut R,
) -> Result<(usize, usize), ForesterError> {
    let merkle_tree = get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
        client,
        *merkle_tree_pubkey,
    )
    .await;
    Ok((merkle_tree.changelog_index(), merkle_tree.sequence_number()))
}
