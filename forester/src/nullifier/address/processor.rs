use crate::errors::ForesterError;
use crate::nullifier::address::pipeline::AddressPipelineStage;
use crate::nullifier::queue_data::ForesterAddressQueueAccountData;
use crate::nullifier::{BackpressureControl, ForesterQueueAccount, PipelineContext};
use crate::operations::fetch_address_queue_data;
use crate::tree_sync::TreeData;
use crate::{ForesterConfig, RpcPool};
use account_compression::utils::constants::{
    ADDRESS_MERKLE_TREE_CHANGELOG, ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG,
};
use light_registry::sdk::{
    create_update_address_merkle_tree_instruction, UpdateAddressMerkleTreeInstructionInputs,
};
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::{debug, error, info, warn};
use rand::seq::SliceRandom;
use rand::thread_rng;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex, Semaphore};

pub struct AddressProcessor<T: Indexer<R>, R: RpcConnection> {
    pub input: mpsc::Receiver<AddressPipelineStage<T, R>>,
    pub output: mpsc::Sender<AddressPipelineStage<T, R>>,
    pub backpressure: BackpressureControl,
    pub shutdown: Arc<AtomicBool>,
    pub close_output: mpsc::Receiver<()>,
    pub address_queue: Arc<Mutex<Vec<ForesterQueueAccount>>>,
}

impl<T: Indexer<R>, R: RpcConnection> AddressProcessor<T, R> {
    pub(crate) async fn process(&mut self) {
        debug!("Starting AddressProcessor process");
        let mut consecutive_errors = 0;
        loop {
            tokio::select! {
                Some(item) = self.input.recv() => {
                    debug!("Received item in AddressProcessor");
                    let _permit = self.backpressure.acquire().await;
                    let result = match item {
                        AddressPipelineStage::FetchAddressQueueData(context) => {
                            info!("Processing FetchAddressQueueData stage");
                            self.fetch_address_queue_data(&context).await
                        }
                        AddressPipelineStage::FetchProofs(context, queue_data) => {
                            info!("Processing FetchAddressQueueData stage");
                            self.fetch_proofs(context, queue_data).await
                        }
                        AddressPipelineStage::UpdateAddressMerkleTree(context, account) => {
                            info!("Processing UpdateAddressMerkleTree stage");
                            self.update_address_merkle_tree(context, account).await
                        }
                        AddressPipelineStage::Complete => {
                            info!("Processing Complete stage");
                            self.shutdown.store(true, Ordering::Relaxed);
                            break;
                        }
                    };

                    match result {
                        Ok(Some(next_stage)) => {
                            info!("Sending next stage: {}", next_stage);
                            if let Err(e) = self.output.send(next_stage).await {
                                warn!("Error sending next stage: {:?}", e);
                                consecutive_errors += 1;
                            }
                        }
                        Ok(None) => {
                            debug!("No next stage to process");
                        }
                        Err(e) => {
                            warn!("Error in AddressProcessor: {:?}", e);
                            consecutive_errors += 1;
                            if consecutive_errors > 5 {
                                error!("Too many consecutive errors, stopping processor");
                                break;
                            }
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        }
                    }
                }
                _ = self.close_output.recv() => {
                    debug!("Received signal to close output channel");
                    break;
                }
                else => break,
            }
            if self.shutdown.load(Ordering::Relaxed) {
                debug!("Shutdown signal received, stopping AddressProcessor");
                break;
            }
        }
        debug!("AddressProcessor process completed");
    }

    async fn fetch_address_queue_data(
        &self,
        context: &PipelineContext<T, R>,
    ) -> Result<Option<AddressPipelineStage<T, R>>, ForesterError> {
        info!("Starting to fetch address queue data");
        let mut address_queue = self.address_queue.lock().await;

        if !address_queue.is_empty() {
            let batch_size = address_queue.len().min(context.config.batch_size);
            let batch: Vec<ForesterQueueAccount> = address_queue.drain(..batch_size).collect();
            return Ok(Some(AddressPipelineStage::FetchProofs(
                context.clone(),
                batch,
            )));
        }

        let mut queue_data = {
            let rpc = context.rpc_pool.get_connection().await;
            let mut queue_data = fetch_address_queue_data(rpc, context.tree_data).await?;
            let mut rng = thread_rng();
            queue_data.accounts.shuffle(&mut rng);
            queue_data
        };

        info!(
            "Fetched address queue data len: {:?}",
            queue_data.accounts.len()
        );
        if queue_data.accounts.is_empty() {
            info!("Address queue is empty");
            Ok(Some(AddressPipelineStage::Complete))
        } else {
            let batch_size = queue_data.accounts.len().min(context.config.batch_size);
            let batch: Vec<ForesterQueueAccount> =
                queue_data.accounts.drain(..batch_size).collect();
            *address_queue = queue_data.accounts;
            Ok(Some(AddressPipelineStage::FetchProofs(
                context.clone(),
                batch,
            )))
        }
    }

    async fn fetch_proofs(
        &self,
        context: PipelineContext<T, R>,
        queue_data: Vec<ForesterQueueAccount>,
    ) -> Result<Option<AddressPipelineStage<T, R>>, ForesterError> {
        let indexer = &context.indexer;

        let addresses = queue_data.iter().map(|account| account.hash).collect();

        let proofs = match indexer
            .lock()
            .await
            .get_multiple_new_address_proofs(context.tree_data.tree_pubkey.to_bytes(), addresses)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                warn!("Error fetching proofs: {:?}", e);
                return Ok(Some(AddressPipelineStage::Complete));
            }
        };

        if proofs.is_empty() {
            return Ok(Some(AddressPipelineStage::FetchAddressQueueData(context)));
        }

        let account_data = queue_data
            .into_iter()
            .zip(proofs.into_iter())
            .map(|(account, proof)| ForesterAddressQueueAccountData { account, proof })
            .collect();

        Ok(Some(AddressPipelineStage::UpdateAddressMerkleTree(
            context,
            account_data,
        )))
    }

    async fn update_address_merkle_tree(
        &self,
        context: PipelineContext<T, R>,
        account_data_batch: Vec<ForesterAddressQueueAccountData>,
    ) -> Result<Option<AddressPipelineStage<T, R>>, ForesterError> {
        let indexer = &context.indexer;

        // Create a channel for collecting results
        let (tx, mut rx) = mpsc::channel(account_data_batch.len());

        // Create a semaphore to limit concurrent tasks
        let semaphore = Arc::new(Semaphore::new(128));

        // Spawn tasks for each account_data
        for account_data in account_data_batch {
            let tx = tx.clone();
            let context = context.clone();
            let semaphore = semaphore.clone();

            tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                let mut retry_count = 0;
                while retry_count < context.config.max_retries {
                    match update_merkle_tree(
                        context.rpc_pool.clone(),
                        context.config.clone(),
                        account_data.clone(),
                        &context.tree_data,
                    )
                    .await
                    {
                        Ok(true) => {
                            tx.send((true, account_data)).await.unwrap();
                            return;
                        }
                        Ok(false) => {
                            retry_count += 1;
                        }
                        Err(e) => {
                            warn!(
                                "Error updating merkle tree for address {:?}: {:?}",
                                account_data.account.hash, e
                            );
                            retry_count += 1;
                        }
                    }

                    if retry_count < context.config.max_retries {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    }
                }
                tx.send((false, account_data)).await.unwrap();
            });
        }
        drop(tx);

        while let Some((success, account_data)) = rx.recv().await {
            if success {
                debug!("Merkle tree updated: {:?}", account_data.account.hash);
                indexer.lock().await.address_tree_updated(
                    context.tree_data.tree_pubkey.to_bytes(),
                    &account_data.proof,
                );
            } else {
                warn!(
                    "Failed to update merkle tree for address: {:?}",
                    account_data.account.hash
                );
            }
        }

        let address_queue = self.address_queue.lock().await;
        if address_queue.is_empty() {
            Ok(Some(AddressPipelineStage::FetchAddressQueueData(context)))
        } else {
            Ok(Some(AddressPipelineStage::FetchProofs(context, Vec::new())))
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn update_merkle_tree<R: RpcConnection>(
    rpc_pool: RpcPool<R>,
    config: Arc<ForesterConfig>,
    account_data: ForesterAddressQueueAccountData,
    tree_data: &TreeData,
) -> Result<bool, ForesterError> {
    let start = Instant::now();

    let update_ix =
        create_update_address_merkle_tree_instruction(UpdateAddressMerkleTreeInstructionInputs {
            authority: config.payer_keypair.pubkey(),
            address_merkle_tree: tree_data.tree_pubkey,
            address_queue: tree_data.queue_pubkey,
            value: account_data.account.index as u16,
            low_address_index: account_data.proof.low_address_index,
            low_address_value: account_data.proof.low_address_value,
            low_address_next_index: account_data.proof.low_address_next_index,
            low_address_next_value: account_data.proof.low_address_next_value,
            low_address_proof: account_data.proof.low_address_proof,
            changelog_index: (account_data.proof.root_seq % ADDRESS_MERKLE_TREE_CHANGELOG) as u16,
            indexed_changelog_index: ((account_data.proof.root_seq - 1)
                % ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG)
                as u16,
        });

    // Prepare the instructions
    let instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(config.cu_limit),
        update_ix,
    ];

    // Acquire the RPC lock only to get the latest blockhash
    let latest_blockhash = {
        let rpc = rpc_pool.get_connection().await;
        let mut rpc_guard = rpc.lock().await;
        rpc_guard.get_latest_blockhash().await?
    };

    let blockhash_time = start.elapsed();
    info!("Time to get blockhash: {:?}", blockhash_time);

    // Create the transaction
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&config.payer_keypair.pubkey()),
        &[&config.payer_keypair],
        latest_blockhash,
    );

    // Acquire the RPC lock again to send and confirm the transaction
    let (signature, confirmed) = {
        let rpc = rpc_pool.get_connection().await;
        let mut rpc_guard = rpc.lock().await;
        let signature = rpc_guard.process_transaction(transaction).await?;
        let confirmed = rpc_guard.confirm_transaction(signature).await?;
        (signature, confirmed)
    };
    // RPC mutex is unlocked here

    let total_time = start.elapsed();
    info!("Total time for transaction: {:?}", total_time);

    info!("Processed: {:?}", signature);
    info!("Confirmed: {:?} {}", signature, confirmed);

    Ok(confirmed)
}
