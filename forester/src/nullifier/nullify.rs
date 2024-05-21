use crate::errors::ForesterError;
use crate::nullifier::queue_data::Account;
use crate::nullifier::{Config, QueueData};
use account_compression::{QueueAccount, StateMerkleTreeAccount};
use anchor_lang::AccountDeserialize;
use light_hash_set::HashSet;
use light_test_utils::indexer::Indexer;
use log::{info, warn};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
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

    let mut queue_data = fetch_queue_data(arc_indexer.clone(), config).await?;
    loop {
        if cancellation_token.is_cancelled() {
            info!("Cancellation detected, exiting loop...");
            break;
        }
        if queue_data.compressed_accounts_to_nullify.is_empty()
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

        if queue_data.compressed_accounts_to_nullify.is_empty() {
            info!("No more accounts to nullify. Exiting...");
            cancellation_token.cancel();
            break;
        }

        let permit = Arc::clone(&semaphore).acquire_owned().await;
        let account = queue_data.compressed_accounts_to_nullify.remove(0);

        let max_retries = config.max_retries;
        if let Some((proof, leaf_index, root_seq)) = queue_data
            .compressed_account_proofs
            .remove(&account.hash_string())
        {
            let client = RpcClient::new(&config.server_url);
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
                    match nullify_compressed_account(
                        account,
                        queue_data.change_log_index,
                        queue_data.sequence_number as i64,
                        proof_clone,
                        leaf_index,
                        root_seq,
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
            queue_data = fetch_queue_data(Arc::clone(&arc_indexer), config).await?;
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
) -> Result<QueueData, ForesterError> {
    let (change_log_index, sequence_number) = {
        let temporary_client = RpcClient::new(&config.server_url);
        get_changelog_index(&config.merkle_tree_pubkey, &temporary_client)?
    };
    let compressed_accounts_to_nullify = {
        let temporary_client = RpcClient::new(&config.server_url);
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
    Ok(QueueData {
        change_log_index,
        sequence_number,
        compressed_accounts_to_nullify,
        compressed_account_proofs,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn nullify_compressed_account<T: Indexer>(
    account: Account,
    change_log_index: usize,
    sequence_number: i64,
    proof: Vec<[u8; 32]>,
    leaf_index: u64,
    root_seq: i64,
    config: &Config,
    client: &RpcClient,
    indexer: Arc<Mutex<T>>,
) -> Result<(), ForesterError> {
    let diff = root_seq - sequence_number;
    let change_log_index =
        change_log_index
            .checked_add(diff as usize)
            .ok_or(ForesterError::Custom(
                "change_log_index overflow".to_string(),
            ))?;

    let time = std::time::Instant::now();
    let instructions = [
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
        account_compression::nullify_leaves::sdk_nullify::create_nullify_instruction(
            vec![change_log_index as u64].as_slice(),
            vec![account.index as u16].as_slice(),
            vec![leaf_index].as_slice(),
            vec![proof].as_slice(),
            &config.payer_keypair.pubkey(),
            &config.merkle_tree_pubkey,
            &config.nullifier_queue_pubkey,
        ),
    ];
    let latest_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&config.payer_keypair.pubkey()),
        &[&config.payer_keypair],
        latest_blockhash,
    );

    let tx_config = RpcSendTransactionConfig {
        skip_preflight: true,
        ..RpcSendTransactionConfig::default()
    };
    let signature = client.send_transaction_with_config(&transaction, tx_config)?;
    loop {
        let confirmed = client.confirm_transaction(&signature).unwrap();
        if confirmed {
            break;
        }
    }

    info!("Account hash: {}, Time: {:?}. Sig: {:?}, index_in_nullifier_queue: {}, root_seq: {}, \
                        sequence_number: {}, diff: {}, old_change_log_index: {}, new_change_log_index: {}, leaf_index: {}",
             account.hash_string(),
             time.elapsed(),
             signature,
             account.index,
             root_seq,
             sequence_number,
             diff,
             change_log_index - diff as usize,
             change_log_index,
             leaf_index);
    let mut guard = indexer.lock().await;
    let indexer = guard.deref_mut();
    indexer.account_nullified(config.merkle_tree_pubkey, &account.hash_string());
    Ok(())
}

pub fn get_nullifier_queue(
    nullifier_queue_pubkey: &Pubkey,
    client: &RpcClient,
) -> Result<Vec<Account>, ForesterError> {
    let mut nullifier_queue_account = client.get_account(nullifier_queue_pubkey)?;
    let nullifier_queue: HashSet<u16> = unsafe {
        HashSet::from_bytes_copy(
            &mut nullifier_queue_account.data[8 + mem::size_of::<QueueAccount>()..],
        )?
    };

    let mut compressed_accounts_to_nullify = Vec::new();
    for (i, element) in nullifier_queue.iter() {
        if element.sequence_number().is_none() {
            compressed_accounts_to_nullify.push(Account {
                hash: element.value_bytes(),
                index: i,
            });
        }
    }
    Ok(compressed_accounts_to_nullify)
}

pub fn get_changelog_index(
    merkle_tree_pubkey: &Pubkey,
    client: &RpcClient,
) -> Result<(usize, usize), ForesterError> {
    let data: &[u8] = &client.get_account_data(merkle_tree_pubkey)?;
    let mut data_ref = data;
    let merkle_tree_account: StateMerkleTreeAccount =
        StateMerkleTreeAccount::try_deserialize(&mut data_ref)?;
    let merkle_tree = merkle_tree_account.copy_merkle_tree()?;
    Ok((
        merkle_tree.current_changelog_index,
        merkle_tree.sequence_number,
    ))
}
