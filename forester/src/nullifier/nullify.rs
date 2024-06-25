use crate::errors::ForesterError;
use crate::nullifier::queue_data::Account;
use crate::nullifier::{Config, StateQueueData};
use account_compression::utils::constants::STATE_MERKLE_TREE_CHANGELOG;
use account_compression::{QueueAccount, StateMerkleTreeAccount};
use futures::future::select_all;
use light_hash_set::HashSet;
use light_hasher::Poseidon;
use light_registry::sdk::{create_nullify_instruction, CreateNullifyInstructionInputs};
use light_test_utils::get_concurrent_merkle_tree;
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::{info, warn};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use std::collections::HashMap;
use std::mem;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

#[allow(dead_code)]
pub async fn pub_nullify<T: Indexer, R: RpcConnection>(
    indexer: Arc<Mutex<T>>,
    rpc: Arc<Mutex<R>>,
    config: Arc<Config>,
) -> Result<(), ForesterError> {
    // TODO: check that our part of the queue is not empty before starting
    // fetch the queue data, pass the data to the nullify function and nullify the accounts
    nullify(indexer, rpc, config).await
}

pub async fn nullify<T: Indexer, R: RpcConnection>(
    indexer: Arc<Mutex<T>>,
    rpc: Arc<Mutex<R>>,
    config: Arc<Config>,
) -> Result<(), ForesterError> {
    // let concurrency_limit = config.concurrency_limit;
    // let semaphore = Arc::new(Semaphore::new(concurrency_limit));
    let successful_nullifications = Arc::new(Mutex::new(0));
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

    let mut handles: Vec<JoinHandle<Result<(), ForesterError>>> = vec![];

    loop {
        if cancellation_token.is_cancelled() {
            info!("Cancellation detected, exiting loop...");
            break;
        }

        // Process completed tasks
        handles.retain(|h: &JoinHandle<Result<(), ForesterError>>| !h.is_finished());

        let config = Arc::clone(&config);
        let config_clone = config.clone();
        while handles.len() >= config.max_concurrent_batches {
            // Wait for at least one task to complete before spawning more
            let (result, index, remaining) = select_all(handles).await;
            handles = remaining;
            if let Err(e) = result {
                warn!("Error processing batch: {:?}", e);
            }
            info!("Task {} completed", index);
        }

        let queue_data = fetch_queue_data(&indexer, &rpc, config).await?;

        if queue_data.is_none() {
            info!("No more accounts to nullify. Exiting...");
            break;
        }

        let queue_data = queue_data.unwrap();
        let successful_nullifications_clone = Arc::clone(&successful_nullifications);
        let cancellation_token_clone = cancellation_token.clone();
        let indexer_clone = Arc::clone(&indexer);
        let rpc_clone = Arc::clone(&rpc);
        let handle: JoinHandle<Result<(), ForesterError>> = tokio::spawn(async move {
            process_batch(
                queue_data,
                &successful_nullifications_clone,
                &cancellation_token_clone,
                &config_clone,
                &rpc_clone,
                &indexer_clone,
            )
            .await
        });

        handles.push(handle);
    }

    // Wait for all remaining tasks to complete
    while !handles.is_empty() {
        let (result, _index, remaining) = select_all(handles).await;
        handles = remaining;
        if let Err(e) = result {
            warn!("Error processing batch: {:?}", e);
        }
    }

    // TODO: should we use terminate_handle.await.unwrap() here?
    // let successful_nullifications = successful_nullifications.lock().await;
    // info!("Successful nullifications: {}", *successful_nullifications);

    Ok(())
}

async fn process_batch<T: Indexer, R: RpcConnection>(
    mut queue_data: StateQueueData,
    successful_nullifications: &Arc<Mutex<usize>>,
    cancellation_token: &CancellationToken,
    config: &Arc<Config>,
    rpc: &Arc<Mutex<R>>,
    indexer: &Arc<Mutex<T>>,
) -> Result<(), ForesterError> {
    while !queue_data.compressed_accounts_to_nullify.is_empty() {
        let account = queue_data.compressed_accounts_to_nullify.remove(0);
        if let Some((proof, leaf_index, root_seq)) = queue_data
            .compressed_account_proofs
            .remove(&account.hash_string())
        {
            let mut retries = 0;
            loop {
                if cancellation_token.is_cancelled() {
                    info!("Task cancelled for account {}", account.hash_string());
                    return Ok(());
                }
                let proof_clone = proof.clone();
                info!("Nullifying account: {}", account.hash_string());
                match nullify_compressed_account(
                    account,
                    queue_data.change_log_index,
                    proof_clone,
                    leaf_index,
                    root_seq,
                    config,
                    &mut *rpc.lock().await,
                    &mut *indexer.lock().await,
                )
                .await
                {
                    Ok(_) => {
                        let mut successful_nullifications = successful_nullifications.lock().await;
                        *successful_nullifications += 1;
                        break;
                    }
                    Err(e) => {
                        if retries >= config.max_retries {
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
        } else {
            warn!("No proof found for account: {}", account.hash_string());
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            continue;
        }
    }
    Ok(())
}

async fn fetch_queue_data<T: Indexer, R: RpcConnection>(
    indexer: &Arc<Mutex<T>>,
    rpc: &Arc<Mutex<R>>,
    config: Arc<Config>,
) -> Result<Option<StateQueueData>, ForesterError> {
    let (change_log_index, sequence_number) =
        { get_changelog_index(&config.state_merkle_tree_pubkey, &mut *rpc.lock().await).await? };
    let compressed_accounts_to_nullify = {
        let queue =
            get_nullifier_queue(&config.nullifier_queue_pubkey, &mut *rpc.lock().await).await?;
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

    info!(
        "Fetching proofs for accounts: {:?}",
        compressed_account_list
    );
    let proofs = &mut *indexer
        .lock()
        .await
        .get_multiple_compressed_account_proofs(compressed_account_list.clone())
        .await
        .map_err(|e| {
            warn!("Cannot get multiple proofs: {:#?}", e);
            ForesterError::NoProofsFound
        })?;
    let compressed_account_proofs: HashMap<String, (Vec<[u8; 32]>, u64, u64)> = proofs
        .iter_mut()
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
pub async fn nullify_compressed_account<T: Indexer, R: RpcConnection>(
    account: Account,
    change_log_index: usize,
    proof: Vec<[u8; 32]>,
    leaf_index: u64,
    root_seq: u64,
    config: &Config,
    rpc: &mut R,
    indexer: &mut T,
) -> Result<(), ForesterError> {
    info!("Nullifying account: {}...", account.hash_string());
    info!("Change log index: {}", change_log_index);
    info!("Leaf index: {}", leaf_index);
    info!("Root seq: {}", root_seq);
    let root_seq_mod = root_seq % STATE_MERKLE_TREE_CHANGELOG;
    info!("Root seq mod: {}", root_seq_mod);

    let ix = create_nullify_instruction(CreateNullifyInstructionInputs {
        nullifier_queue: config.nullifier_queue_pubkey,
        merkle_tree: config.state_merkle_tree_pubkey,
        change_log_indices: vec![root_seq_mod],
        leaves_queue_indices: vec![account.index as u16],
        indices: vec![leaf_index],
        proofs: vec![proof],
        authority: config.payer_keypair.pubkey(),
        derivation: Pubkey::from_str("En9a97stB3Ek2n6Ey3NJwCUJnmTzLMMEA5C69upGDuQP").unwrap(),
    });
    let instructions = [
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
        ix,
    ];
    info!("Authority: {:?}", config.payer_keypair.pubkey());
    let signature = rpc
        .create_and_send_transaction(
            &instructions,
            &config.payer_keypair.pubkey(),
            &[&config.payer_keypair],
        )
        .await;
    info!("Transaction: {:?}", signature);

    // TODO: check if the transaction was successful and implement retry logic

    indexer.account_nullified(config.state_merkle_tree_pubkey, &account.hash_string());
    Ok(())
}

pub async fn get_nullifier_queue<R: RpcConnection>(
    nullifier_queue_pubkey: &Pubkey,
    rpc: &mut R,
) -> Result<Vec<Account>, ForesterError> {
    let mut nullifier_queue_account = rpc
        .get_account(*nullifier_queue_pubkey)
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
    rpc: &mut R,
) -> Result<(usize, usize), ForesterError> {
    let merkle_tree = get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
        rpc,
        *merkle_tree_pubkey,
    )
    .await;
    Ok((merkle_tree.changelog_index(), merkle_tree.sequence_number()))
}
