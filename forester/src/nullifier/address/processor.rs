use crate::errors::ForesterError;
use crate::nullifier::address::pipeline::AddressPipelineStage;
use crate::nullifier::{BackpressureControl, ForesterQueueAccount, PipelineContext};
use account_compression::utils::constants::{
    ADDRESS_MERKLE_TREE_CHANGELOG, ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG,
};
use light_registry::sdk::{
    create_update_address_merkle_tree_instruction, UpdateAddressMerkleTreeInstructionInputs,
};
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::{debug, error, info, warn};
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Semaphore};
use rand::seq::SliceRandom;
use rand::thread_rng;
use crate::nullifier::queue_data::ForesterAddressQueueAccountData;
use crate::operations::{fetch_address_queue_data};

pub struct AddressProcessor<T: Indexer<R>, R: RpcConnection> {
    pub input: mpsc::Receiver<AddressPipelineStage<T, R>>,
    pub output: mpsc::Sender<AddressPipelineStage<T, R>>,
    pub backpressure: BackpressureControl,
    pub shutdown: Arc<AtomicBool>,
    pub close_output: mpsc::Receiver<()>,
    pub address_queue: Arc<Mutex<Vec<ForesterQueueAccount>>>
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
            return Ok(Some(AddressPipelineStage::FetchProofs(context.clone(), batch)));
        }

        let mut queue_data = fetch_address_queue_data(context.config.clone(), context.rpc.clone()).await?;
        let mut rng = thread_rng();
        queue_data.shuffle(&mut rng);

        info!("Fetched address queue data len: {:?}", queue_data.len());
        if queue_data.is_empty() {
            info!("Address queue is empty");
            Ok(Some(AddressPipelineStage::Complete))
        } else {
            let batch_size = queue_data.len().min(context.config.batch_size);
            let batch: Vec<ForesterQueueAccount> = queue_data.drain(..batch_size).collect();
            *address_queue = queue_data;
            Ok(Some(AddressPipelineStage::FetchProofs(context.clone(), batch)))
        }

        /*

        let mut queue_data = fetch_address_queue_data(context.config.clone(), context.rpc.clone()).await?;
        let mut rng = thread_rng();
        queue_data.shuffle(&mut rng);

        info!("Fetched address queue data len: {:?}", queue_data.len());
        if queue_data.is_empty() {
            info!("Address queue is empty");
            Ok(Some(AddressPipelineStage::Complete))
        } else {
            let batch: Vec<ForesterQueueAccount> = queue_data.into_iter().take(context.config.batch_size).collect();
            info!("Processing batch of {} addresses", batch.len());
            Ok(Some(AddressPipelineStage::FetchProofs(
                context.clone(),
                batch,
            )))
        }
        */
    }

    async fn fetch_proofs(
        &self,
        context: PipelineContext<T, R>,
        queue_data: Vec<ForesterQueueAccount>,
    ) -> Result<Option<AddressPipelineStage<T, R>>, ForesterError> {
        let indexer = &context.indexer;

        let addresses = queue_data.iter().map(|account| account.hash).collect();

        let proofs = match indexer.lock().await.get_multiple_new_address_proofs(context.config.address_merkle_tree_pubkey.to_bytes(), addresses).await {
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
            .map(|(account, proof)| {
                ForesterAddressQueueAccountData {
                    account,
                    proof
                }
            })
            .collect();

        Ok(Some(AddressPipelineStage::UpdateAddressMerkleTree(context, account_data)))
    }


    async fn update_address_merkle_tree(
        &self,
        context: PipelineContext<T, R>,
        account_data_batch: Vec<ForesterAddressQueueAccountData>,
    ) -> Result<Option<AddressPipelineStage<T, R>>, ForesterError> {
        let config = &context.config;
        let indexer = &context.indexer;

        // Create a channel for collecting results
        let (tx, mut rx) = mpsc::channel(account_data_batch.len());

        // Create a semaphore to limit concurrent tasks
        let semaphore = Arc::new(Semaphore::new(10));

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
                        &context.rpc.clone(),
                        &context.config.payer_keypair,
                        context.config.address_merkle_tree_queue_pubkey,
                        context.config.address_merkle_tree_pubkey,
                        account_data.account.index as u16,
                        account_data.proof.low_address_index,
                        account_data.proof.low_address_value,
                        account_data.proof.low_address_next_index,
                        account_data.proof.low_address_next_value,
                        account_data.proof.low_address_proof,
                        (account_data.proof.root_seq % ADDRESS_MERKLE_TREE_CHANGELOG) as u16,
                        ((account_data.proof.root_seq - 1) % ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG) as u16,
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

        let mut successful_updates = 0;
        while let Some((success, account_data)) = rx.recv().await {
            if success {
                successful_updates += 1;
                debug!(
                "Successfully updated merkle tree for address: {:?}",
                account_data.account.hash
            );
                indexer
                    .lock()
                    .await
                    .address_tree_updated(config.address_merkle_tree_pubkey.to_bytes(), &account_data.proof);
            } else {
                warn!(
                "Failed to update merkle tree for address: {:?}",
                account_data.account.hash
            );
            }
        }

        let mut nullifications = context.successful_nullifications.lock().await;
        *nullifications += successful_updates;
        info!("Nullifications: {:?}", *nullifications);

        if *nullifications >= (ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG / 2) as usize {
            info!(
            "Reached {} successful nullifications. Re-fetching queue.",
            *nullifications
        );
            *nullifications = 0;
            drop(nullifications);
            return Ok(Some(AddressPipelineStage::FetchAddressQueueData(
                context.clone(),
            )));
        }
        drop(nullifications);

        // Ok(Some(AddressPipelineStage::FetchAddressQueueData(context)))

        let address_queue = self.address_queue.lock().await;
        if address_queue.is_empty() {
            Ok(Some(AddressPipelineStage::FetchAddressQueueData(context)))
        } else {
            Ok(Some(AddressPipelineStage::FetchProofs(context, Vec::new())))
        }
    }

    //
    // async fn update_address_merkle_tree(
    //     &self,
    //     context: PipelineContext<T, R>,
    //     account_data_batch: Vec<ForesterAddressQueueAccountData>,
    // ) -> Result<Option<AddressPipelineStage<T, R>>, ForesterError> {
    //     let config = &context.config;
    //     let rpc = &context.rpc;
    //     let indexer = &context.indexer;
    //     let mut retry_count = 0;
    //
    //     for account_data in account_data_batch {
    //         while retry_count < context.config.max_retries {
    //             match update_merkle_tree(
    //                 rpc,
    //                 &config.payer_keypair,
    //                 config.address_merkle_tree_queue_pubkey,
    //                 config.address_merkle_tree_pubkey,
    //                 account_data.account.index as u16,
    //                 account_data.proof.low_address_index,
    //                 account_data.proof.low_address_value,
    //                 account_data.proof.low_address_next_index,
    //                 account_data.proof.low_address_next_value,
    //                 account_data.proof.low_address_proof,
    //                 // TODO: use changelog array size from tree config
    //                 (account_data.proof.root_seq % ADDRESS_MERKLE_TREE_CHANGELOG) as u16,
    //                 // TODO:
    //                 // 1. add index changelog current changelog index to the proof or we make them the same size
    //                 // 2. remove -1 after new zktestnet release
    //                 ((account_data.proof.root_seq - 1) % ADDRESS_MERKLE_TREE_CHANGELOG) as u16,
    //             )
    //                 .await
    //             {
    //                 Ok(true) => {
    //                     debug!(
    //                     "Successfully updated merkle tree for address: {:?}",
    //                     account_data.account.hash
    //                 );
    //                     let mut nullifications = context.successful_nullifications.lock().await;
    //                     *nullifications += 1;
    //                     info!("Nullifications: {:?}", *nullifications);
    //
    //                     indexer
    //                         .lock()
    //                         .await
    //                         .address_tree_updated(config.address_merkle_tree_pubkey.to_bytes(), &account_data.proof);
    //
    //                     if *nullifications >= (ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG / 2) as usize {
    //                         info!(
    //                         "Reached {} successful nullifications. Re-fetching queue.",
    //                         *nullifications
    //                     );
    //                         *nullifications = 0;
    //                         drop(nullifications);
    //                         return Ok(Some(AddressPipelineStage::FetchAddressQueueData(
    //                             context.clone(),
    //                         )));
    //                     }
    //                     drop(nullifications);
    //                     return Ok(Some(AddressPipelineStage::FetchAddressQueueData(
    //                         context.clone(),
    //                     )));
    //                 }
    //                 Ok(false) => {
    //                     warn!("Failed to update merkle tree for address: {:?}", account_data.account.hash);
    //                     retry_count += 1;
    //                 }
    //                 Err(e) => {
    //                     warn!(
    //                     "Error updating merkle tree for address {:?}: {:?}",
    //                     account_data.account.hash, e
    //                 );
    //                     retry_count += 1;
    //                 }
    //             }
    //
    //             if retry_count < context.config.max_retries {
    //                 debug!(
    //                 "Retrying update for address: {:?} (Attempt {} of {})",
    //                 account_data.account.hash,
    //                 retry_count + 1,
    //                 context.config.max_retries
    //             );
    //                 tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    //             }
    //         }
    //
    //         warn!(
    //         "Max retries reached for address: {:?}. Moving to next address.",
    //         account_data.account.hash
    //     );
    //     }
    //     Ok(Some(AddressPipelineStage::FetchAddressQueueData(context)))
    // }
}


#[allow(clippy::too_many_arguments)]
pub async fn update_merkle_tree<R: RpcConnection>(
    rpc: &Arc<Mutex<R>>,
    payer: &Keypair,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    value: u16,
    low_address_index: u64,
    low_address_value: [u8; 32],
    low_address_next_index: u64,
    low_address_next_value: [u8; 32],
    low_address_proof: [[u8; 32]; 16],
    changelog_index: u16,
    indexed_changelog_index: u16,
) -> Result<bool, ForesterError> {
    debug!("changelog_index: {:?}", changelog_index);
    debug!("indexed_changelog_index: {:?}", indexed_changelog_index);

    // let (onchain_changelog_index, onchain_indexed_changelog_index) = get_address_account_changelog_indices(&address_merkle_tree_pubkey, &mut *rpc.lock().await).await?;
    // info!("onchain changelog_index: {:?}", onchain_changelog_index);
    // info!("onchain indexed_changelog_index: {:?}", onchain_indexed_changelog_index);

    let update_ix =
        create_update_address_merkle_tree_instruction(UpdateAddressMerkleTreeInstructionInputs {
            authority: payer.pubkey(),
            address_merkle_tree: address_merkle_tree_pubkey,
            address_queue: address_queue_pubkey,
            value,
            low_address_index,
            low_address_value,
            low_address_next_index,
            low_address_next_value,
            low_address_proof,
            changelog_index,
            indexed_changelog_index,
        });
    info!("Sending transaction...");

    let rpc = &mut *rpc.lock().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
            update_ix,
        ],
        Some(&payer.pubkey()),
        &[&payer],
        rpc.get_latest_blockhash().await.unwrap(),
    );

    let signature = rpc.process_transaction(transaction).await?;
    info!("Signature: {:?}", signature);
    let confirmed = rpc.confirm_transaction(signature).await?;
    info!("Confirmed: {:?}", confirmed);
    Ok(confirmed)
}
