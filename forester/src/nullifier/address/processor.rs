use crate::config::ForesterConfig;
use crate::errors::ForesterError;
use crate::nullifier::address::pipeline::AddressPipelineStage;
use crate::nullifier::{BackpressureControl, ForesterQueueAccount, PipelineContext};
use account_compression::utils::constants::{
    ADDRESS_MERKLE_TREE_CHANGELOG, ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG,
};
use account_compression::{AddressMerkleTreeAccount, QueueAccount};
use light_hash_set::HashSet;
use light_hasher::Poseidon;
use light_registry::sdk::{
    create_update_address_merkle_tree_instruction, UpdateAddressMerkleTreeInstructionInputs,
};
use light_test_utils::get_indexed_merkle_tree;
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::{debug, info, warn};
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use rand::seq::SliceRandom;
use rand::thread_rng;

pub struct AddressProcessor<T: Indexer<R>, R: RpcConnection> {
    pub input: mpsc::Receiver<AddressPipelineStage<T, R>>,
    pub output: mpsc::Sender<AddressPipelineStage<T, R>>,
    pub backpressure: BackpressureControl,
    pub shutdown: Arc<AtomicBool>,
    pub close_output: mpsc::Receiver<()>,
}

impl<T: Indexer<R>, R: RpcConnection> AddressProcessor<T, R> {
    pub(crate) async fn process(&mut self) {
        debug!("Starting AddressProcessor process");
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
                        AddressPipelineStage::ProcessAddressQueue(context, queue_data) => {
                            info!("Processing ProcessAddressQueue stage");
                            self.process_address_queue(context, queue_data).await
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
                            }
                        }
                        Ok(None) => {
                            debug!("No next stage to process");
                        }
                        Err(e) => {
                            warn!("Error in AddressProcessor: {:?}", e);
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
        let config = &context.config;
        let rpc = &context.rpc;

        let mut queue_data = fetch_address_queue_data(config, rpc).await?;
        let mut rng = thread_rng();
        queue_data.shuffle(&mut rng);
        
        info!("Fetched address queue data len: {:?}", queue_data.len());
        if queue_data.is_empty() {
            info!("Address queue is empty");
            Ok(Some(AddressPipelineStage::Complete))
        } else {
            info!("Processing to process address queue");
            Ok(Some(AddressPipelineStage::ProcessAddressQueue(
                context.clone(),
                queue_data,
            )))
        }
    }

    async fn process_address_queue(
        &self,
        context: PipelineContext<T, R>,
        queue_data: Vec<ForesterQueueAccount>,
    ) -> Result<Option<AddressPipelineStage<T, R>>, ForesterError> {        
        if let Some(account) = queue_data.first() {
            info!("Processing address: {:?}", account.hash);
            Ok(Some(AddressPipelineStage::UpdateAddressMerkleTree(
                context, *account,
            )))
        } else {
            Ok(None)
        }
    }

    async fn update_address_merkle_tree(
        &self,
        context: PipelineContext<T, R>,
        account: ForesterQueueAccount,
    ) -> Result<Option<AddressPipelineStage<T, R>>, ForesterError> {
        let config = &context.config;
        let rpc = &context.rpc;
        let indexer = &context.indexer;

        let address = account.hash;
        let address_hashset_index = account.index;

        let proof = indexer
            .lock()
            .await
            .get_multiple_new_address_proofs(config.address_merkle_tree_pubkey.to_bytes(), address)
            .await
            .map_err(|_| ForesterError::Custom("Failed to get address tree proof".to_string()))?;
        // TODO: use changelog array size from tree config
        let indexer_changelog = proof.root_seq % ADDRESS_MERKLE_TREE_CHANGELOG;
        // TODO:
        // 1. add index changelog current changelog index to the proof or we make them the same size
        // 2. remove -1 after new zktestnet release
        let indexer_index_changelog = (proof.root_seq - 1) % ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG;

        debug!("changelog: {:?}", indexer_changelog);
        debug!("index_changelog: {:?}", indexer_index_changelog);

        let mut retry_count = 0;

        while retry_count < context.config.max_retries {
            match update_merkle_tree(
                rpc,
                &config.payer_keypair,
                config.address_merkle_tree_queue_pubkey,
                config.address_merkle_tree_pubkey,
                address_hashset_index as u16,
                proof.low_address_index,
                proof.low_address_value,
                proof.low_address_next_index,
                proof.low_address_next_value,
                proof.low_address_proof,
                indexer_changelog as u16,
                indexer_index_changelog as u16,
            )
            .await
            {
                Ok(true) => {
                    debug!(
                        "Successfully updated merkle tree for address: {:?}",
                        address
                    );
                    let mut nullifications = context.successful_nullifications.lock().await;
                    *nullifications += 1;
                    info!("Nullifications: {:?}", *nullifications);

                    indexer
                        .lock()
                        .await
                        .address_tree_updated(config.address_merkle_tree_pubkey.to_bytes(), &proof);

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
                    return Ok(Some(AddressPipelineStage::FetchAddressQueueData(
                        context.clone(),
                    )));
                }
                Ok(false) => {
                    warn!("Failed to update merkle tree for address: {:?}", address);
                    retry_count += 1;
                }
                Err(e) => {
                    warn!(
                        "Error updating merkle tree for address {:?}: {:?}",
                        address, e
                    );
                    retry_count += 1;
                }
            }

            if retry_count < context.config.max_retries {
                debug!(
                    "Retrying update for address: {:?} (Attempt {} of {})",
                    address,
                    retry_count + 1,
                    context.config.max_retries
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }

        warn!(
            "Max retries reached for address: {:?}. Moving to next address.",
            address
        );
        Ok(Some(AddressPipelineStage::FetchAddressQueueData(context)))
    }
}

async fn fetch_address_queue_data<R: RpcConnection>(
    config: &Arc<ForesterConfig>,
    rpc: &Arc<Mutex<R>>,
) -> Result<Vec<ForesterQueueAccount>, ForesterError> {
    let address_queue_pubkey = config.address_merkle_tree_queue_pubkey;

    let mut account = (*rpc.lock().await)
        .get_account(address_queue_pubkey)
        .await?
        .unwrap();
    let address_queue: HashSet = unsafe {
        HashSet::from_bytes_copy(&mut account.data[8 + mem::size_of::<QueueAccount>()..])?
    };

    // for i in 0..address_queue.capacity {
    //     let bucket = address_queue.get_bucket(i).unwrap();
    //     if let Some(bucket) = bucket {
    //         info!("{} {:?} {:?}", i, bucket.sequence_number(), bucket.value_bytes());
    //     } else {
    //         info!("{} None ---------------------------------------", i);
    //     }
    // }
    // panic!("stop");
    let mut address_queue_vec = Vec::new();

    for i in 0..address_queue.capacity {
        let bucket = address_queue.get_bucket(i).unwrap();
        if let Some(bucket) = bucket {
            if bucket.sequence_number.is_none() {
                address_queue_vec.push(ForesterQueueAccount {
                    hash: bucket.value_bytes(),
                    index: i,
                });
            }
        }
    }

    Ok(address_queue_vec)
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

pub async fn get_changelog_indices<R: RpcConnection>(
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
