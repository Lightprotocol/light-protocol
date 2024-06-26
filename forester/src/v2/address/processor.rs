use std::mem;
use std::sync::Arc;
use log::{info, warn};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use tokio::sync::{mpsc, Mutex};
use account_compression::{AddressMerkleTreeAccount, QueueAccount};
use light_hash_set::HashSet;
use light_hasher::Poseidon;
use light_registry::sdk::{create_nullify_instruction, create_update_address_merkle_tree_instruction, CreateNullifyInstructionInputs, UpdateAddressMerkleTreeInstructionInputs};
use light_test_utils::{get_concurrent_merkle_tree, get_indexed_merkle_tree};
use light_test_utils::indexer::{Indexer, MerkleProofWithAddressContext};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use crate::errors::ForesterError;
use crate::nullifier::Config;
use crate::v2::BackpressureControl;
use crate::v2::address::pipeline::{PipelineContext, AddressPipelineStage};
use crate::v2::address::queue_data::{AccountData, QueueData};

pub struct AddressProcessor<T: Indexer, R: RpcConnection> {
    pub input: mpsc::Receiver<AddressPipelineStage<T, R>>,
    pub output: mpsc::Sender<AddressPipelineStage<T, R>>,
    pub backpressure: BackpressureControl,
}

impl<T: Indexer, R: RpcConnection> AddressProcessor<T, R> {
    pub(crate) async fn process(&mut self) {
        info!("Starting AddressProcessor process");
        while let Some(item) = self.input.recv().await {
            info!("Received item in AddressProcessor"); //: {:?}", item);
            let _permit = self.backpressure.acquire().await;
            let result = match item {
                AddressPipelineStage::FetchAddressQueueData(context) => {
                    self.fetch_address_queue_data(&context).await
                }
                AddressPipelineStage::ProcessAddressQueue(context, queue_data) => {
                    self.process_address_queue(context, queue_data).await
                }
                AddressPipelineStage::UpdateAddressMerkleTree(context, account) => {
                    self.update_address_merkle_tree(context, account).await
                }
                AddressPipelineStage::UpdateIndexer(context, proof) => {
                    self.update_indexer(context, proof).await
                }
            };

            match result {
                Ok(next_stages) => {
                    for next_stage in next_stages {
                        self.output.send(next_stage).await.unwrap();
                    }
                }
                Err(e) => {
                    warn!("Error in AddressStreamProcessor: {:?}", e);
                }
            }
        }
        info!("StreamProcessor process completed");
    }


    async fn fetch_address_queue_data(&self, context: &PipelineContext<T, R>) 
        -> Result<Vec<AddressPipelineStage<T, R>>, ForesterError> {
        let config = &context.config;
        let rpc = &context.rpc;

        let queue_data = fetch_address_queue_data(config, rpc).await?;
        if queue_data.is_empty() {
            info!("Address queue is empty");
            Ok(vec![])
        } else {
            Ok(vec![AddressPipelineStage::ProcessAddressQueue(context.clone(), queue_data)])
        }
    }

    async fn process_address_queue(&self, context: PipelineContext<T, R>, queue_data: Vec<crate::v2::address::Account>) 
        -> Result<Vec<AddressPipelineStage<T, R>>, ForesterError> {
        let mut next_stages = Vec::new();
        for account in queue_data {
            next_stages.push(AddressPipelineStage::UpdateAddressMerkleTree(context.clone(), account));
        }
        Ok(next_stages)
    }

    async fn update_address_merkle_tree(&self, context: PipelineContext<T, R>, account: crate::v2::address::Account) 
        -> Result<Vec<AddressPipelineStage<T, R>>, ForesterError> {
        let config = &context.config;
        let rpc = &context.rpc;
        let indexer = &context.indexer;

        let address = account.hash;
        let address_hashset_index = account.index;

        let merkle_tree = get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26>(
            &mut *rpc.lock().await,
            config.address_merkle_tree_pubkey,
        ).await;

        let proof = indexer.lock().await
            .get_address_tree_proof(config.address_merkle_tree_pubkey.to_bytes(), address)
            .await
            .map_err(|_| ForesterError::Custom("Failed to get address tree proof".to_string()))?;
            
        let update_successful = update_merkle_tree(
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
        ).await?;

        if update_successful {
            // Ok(vec![AddressPipelineStage::UpdateIndexer(context, proof.into())])
            Ok(vec![])
        } else {
            Err(ForesterError::Custom("Failed to update address merkle tree".to_string()))
        }
    }

    async fn update_indexer(&self, context: PipelineContext<T, R>, proof: MerkleProofWithAddressContext) 
        -> Result<Vec<AddressPipelineStage<T, R>>, ForesterError> {
        let config = &context.config;
        let indexer = &context.indexer;

        indexer.lock().await.address_tree_updated(
            config.address_merkle_tree_pubkey.to_bytes(),
            proof,
        );

        Ok(vec![AddressPipelineStage::FetchAddressQueueData(context)])
    }
}


async fn fetch_address_queue_data<R: RpcConnection>(
    config: &Arc<Config>,
    rpc: &Arc<Mutex<R>>,
) -> Result<Vec<crate::v2::address::Account>, ForesterError> {
    // let address_merkle_tree_pubkey = config.address_merkle_tree_pubkey;
    let address_queue_pubkey = config.address_merkle_tree_queue_pubkey;

    let mut account = (*rpc.lock().await)
        .get_account(address_queue_pubkey)
        .await?
        .unwrap();
    let address_queue: HashSet = unsafe {
        HashSet::from_bytes_copy(&mut account.data[8 + mem::size_of::<QueueAccount>()..])?
    };
    let mut address_queue_vec = Vec::new();
    let address = address_queue.first_no_seq().unwrap();
    info!("address_queue: {:?}", address);
    if address.is_none() {
        return Ok(address_queue_vec);
    }
    let (address, address_hashset_index) = address.unwrap();
    info!("address: {:?}", address);
    info!("address_hashset_index: {:?}", address_hashset_index);
    address_queue_vec.push(crate::v2::address::Account {
        hash: address.value_bytes(),
        index: address_hashset_index as usize,
    });
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
) -> Result<bool, ForesterError> {
    info!("update_merkle_tree");
    let (changelog_index, indexed_changelog_index) =
        get_changelog_indices(&address_merkle_tree_pubkey, &mut *rpc.lock().await)
            .await
            .unwrap();
    info!("changelog_index: {:?}", changelog_index);

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
            changelog_index: changelog_index as u16,
            indexed_changelog_index: indexed_changelog_index as u16,
        });
    info!("sending transaction...");

    let rpc = &mut *rpc.lock().await;
    let transaction = Transaction::new_signed_with_payer(
        &[update_ix],
        Some(&payer.pubkey()),
        &[&payer],
        rpc.get_latest_blockhash().await.unwrap(),
    );

    let signature = rpc.process_transaction(transaction).await?;
    let confirmed = rpc.confirm_transaction(signature).await?;
    Ok(confirmed)
}

pub async fn get_changelog_indices<R: RpcConnection>(
    merkle_tree_pubkey: &Pubkey,
    client: &mut R,
) -> Result<(usize, usize), ForesterError> {
    let merkle_tree = get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26>(
        client,
        *merkle_tree_pubkey,
    )
    .await;
    let changelog_index = merkle_tree.changelog_index();
    let indexed_changelog_index = merkle_tree.indexed_changelog_index();
    Ok((changelog_index, indexed_changelog_index))
}