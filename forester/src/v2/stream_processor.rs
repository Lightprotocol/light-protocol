use std::mem;
use std::str::FromStr;
use log::{info, warn};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use tokio::sync::mpsc;
use account_compression::{QueueAccount, StateMerkleTreeAccount};
use account_compression::utils::constants::STATE_MERKLE_TREE_CHANGELOG;
use light_hash_set::HashSet;
use light_hasher::Poseidon;
use light_registry::sdk::{create_nullify_instruction, CreateNullifyInstructionInputs};
use light_test_utils::get_concurrent_merkle_tree;
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use crate::errors::ForesterError;
use crate::v2::backpressure::BackpressureControl;
use crate::v2::pipeline::{PipelineContext, PipelineStage};
use crate::v2::queue_data::{AccountData, QueueData};

pub struct StreamProcessor<T: Indexer, R: RpcConnection> {
    pub input: mpsc::Receiver<PipelineStage<T, R>>,
    pub output: mpsc::Sender<PipelineStage<T, R>>,
    pub backpressure: BackpressureControl,
}

impl<T: Indexer, R: RpcConnection> StreamProcessor<T, R> {
    pub(crate) async fn process(&mut self) {
        info!("Starting StreamProcessor process");
        while let Some(item) = self.input.recv().await {
            info!("Received item in StreamProcessor"); //: {:?}", item);
            let _permit = self.backpressure.acquire().await;
            let result = match item {
                PipelineStage::FetchQueueData(context) => {
                    info!("Processing FetchQueueData");
                    match StreamProcessor::fetch_queue_data(context).await {
                        Ok(next_stage) => {
                            info!("FetchQueueData successful");
                            vec![next_stage]
                        },
                        Err(e) => {
                            warn!("Error in FetchQueueData: {:?}", e);
                            vec![]
                        }
                    }
                },
                PipelineStage::FetchProofs(context, queue_data) => {
                    info!("Processing FetchProofs");
                    match self.fetch_proofs(context, queue_data).await {
                        Ok(next_stages) => {
                            info!("FetchProofs successful, generated {} next stages", next_stages.len());
                            next_stages
                        },
                        Err(e) => {
                            warn!("Error in FetchProofs: {:?}", e);
                            vec![]
                        }
                    }
                },
                PipelineStage::NullifyAccount(context, account_data) => {
                    let hash = account_data.account.hash_string();
                    info!("Processing NullifyAccount for account: {}", hash);
                    match self.nullify_account(context, account_data).await {
                        Ok(next_stage) => {
                            info!("NullifyAccount successful for account: {}, moving to next stage", hash);
                            vec![next_stage]
                        },
                        Err(e) => {
                            warn!("Error in NullifyAccount for account: {}: {:?}", hash, e);
                            vec![]
                        }
                    }
                },
                PipelineStage::UpdateIndexer(context, account_data) => {
                    let hash = account_data.account.hash_string();
                    info!("Processing UpdateIndexer for account: {}", hash);
                    match self.update_indexer(context, account_data).await {
                        Ok(next_stage) => {
                            info!("UpdateIndexer successful for account: {}, moving to next stage", hash);
                            vec![next_stage]
                        },
                        Err(e) => {
                            warn!("Error in UpdateIndexer for account: {}: {:?}", hash, e);
                            vec![]
                        }
                    }
                },
            };

            info!("Number of next stages: {}", result.len());
            for next_stage in result {
                info!("Attempting to send next stage to output"); //: {:?}", next_stage);
                match self.output.send(next_stage).await {
                    Ok(_) => info!("Successfully sent next stage to output"),
                    Err(e) => warn!("Failed to send next stage to output: {:?}", e),
                }
            }
        }
        info!("StreamProcessor process completed");
    }

    pub(crate) async fn fetch_queue_data(context: PipelineContext<T, R>)
                                         -> Result<PipelineStage<T, R>, ForesterError> {
        let PipelineContext { indexer: _, rpc, config } = &context;

        let (change_log_index, sequence_number) = {
            let mut rpc_lock = rpc.lock().await;
            get_changelog_index(&config.state_merkle_tree_pubkey, &mut *rpc_lock).await?
        };

        let accounts_to_nullify: Vec<AccountData> = {
            let mut rpc_lock = rpc.lock().await;
            let queue = get_nullifier_queue(&config.nullifier_queue_pubkey, &mut *rpc_lock).await?;
            queue.into_iter().take(config.batch_size).collect()
        };

        if accounts_to_nullify.is_empty() {
            info!("No accounts to nullify found in queue");
            return Ok(PipelineStage::FetchProofs(context.clone(), QueueData::new(change_log_index, sequence_number, vec![])));
        }

        let queue_data = QueueData::new(change_log_index, sequence_number, accounts_to_nullify);
        Ok(PipelineStage::FetchProofs(context, queue_data))
    }

    async fn fetch_proofs(&self, context: PipelineContext<T, R>, queue_data: QueueData)
                          -> Result<Vec<PipelineStage<T, R>>, ForesterError> {
        let PipelineContext { indexer, rpc: _, config: _ } = &context;
        let mut next_stages = Vec::new();

        let compressed_account_list: Vec<String> = queue_data.accounts_to_nullify
            .iter()
            .map(|account_data| account_data.account.hash_string())
            .collect();

        let proofs = indexer.lock().await
            .get_multiple_compressed_account_proofs(compressed_account_list)
            .await
            .map_err(|e| {
                warn!("Cannot get multiple proofs: {:#?}", e);
                ForesterError::NoProofsFound
            })?;

        info!("Received {} proofs", proofs.len());

        for (account, proof) in queue_data.accounts_to_nullify.into_iter().zip(proofs.into_iter()) {
            let account_data = AccountData {
                account: account.account,
                proof: proof.proof,
                leaf_index: proof.leaf_index as u64,
                root_seq: proof.root_seq,
            };
            info!("Creating NullifyAccount stage for account: {}", account_data.account.hash_string());
            next_stages.push(PipelineStage::NullifyAccount(context.clone(), account_data));
        }

        info!("Created {} NullifyAccount stages", next_stages.len());
        Ok(next_stages)
    }

    async fn nullify_account(&self, context: PipelineContext<T, R>, account_data: AccountData)
                             -> Result<PipelineStage<T, R>, ForesterError> {
        let PipelineContext { indexer, rpc, config } = &context;

        info!("Nullifying account: {}", account_data.account.hash_string());
        info!("Leaf index: {}", account_data.leaf_index);
        info!("Root seq: {}", account_data.root_seq);

        let root_seq_mod = account_data.root_seq % STATE_MERKLE_TREE_CHANGELOG;
        info!("Root seq mod: {}", root_seq_mod);

        let ix = create_nullify_instruction(CreateNullifyInstructionInputs {
            nullifier_queue: config.nullifier_queue_pubkey,
            merkle_tree: config.state_merkle_tree_pubkey,
            change_log_indices: vec![root_seq_mod],
            leaves_queue_indices: vec![account_data.account.index as u16],
            indices: vec![account_data.leaf_index],
            proofs: vec![account_data.proof.clone()],
            authority: config.payer_keypair.pubkey(),
            derivation: Pubkey::from_str(&config.external_services.derivation).unwrap(),
        });

        let instructions = [
            solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
            ix,
        ];

        info!("Authority: {:?}", config.payer_keypair.pubkey());
        info!("Sending nullification transaction for account: {}", account_data.account.hash_string());
        let signature = rpc.lock().await
            .create_and_send_transaction(
                &instructions,
                &config.payer_keypair.pubkey(),
                &[&config.payer_keypair],
            )
            .await;

        match signature {
            Ok(sig) => {
                info!("Nullification transaction sent successfully for account: {}. Signature: {}", account_data.account.hash_string(), sig);
                info!("Moving to UpdateIndexer stage for account: {}", account_data.account.hash_string());
                Ok(PipelineStage::UpdateIndexer(context.clone(), account_data))
            },
            Err(e) => {
                // TODO: Retry logic
                warn!("Failed to send nullification transaction for account: {}. Error: {:?}", account_data.account.hash_string(), e);
                Err(ForesterError::Custom(format!("Nullification transaction failed: {:?}", e)))
            }
        }
    }

    async fn update_indexer(&self, context: PipelineContext<T, R>, account_data: AccountData)
                            -> Result<PipelineStage<T, R>, ForesterError> {
        let PipelineContext { indexer, rpc: _, config } = &context;

        info!("Updating indexer for account: {}", account_data.account.hash_string());

        indexer.lock().await.account_nullified(
            config.state_merkle_tree_pubkey,
            &account_data.account.hash_string()
        );

        info!("Indexer updated successfully for account: {}", account_data.account.hash_string());
        info!("Completed processing for account: {}, returning to FetchQueueData", account_data.account.hash_string());

        // Since this is the last stage, we'll return to FetchQueueData to start the process again
        Ok(PipelineStage::FetchQueueData(context))
    }
}


pub async fn get_nullifier_queue<R: RpcConnection>(
    nullifier_queue_pubkey: &Pubkey,
    rpc: &mut R,
) -> Result<Vec<AccountData>, ForesterError> {
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
    let mut accounts_to_nullify = Vec::new();
    for i in 0..nullifier_queue.capacity {
        let bucket = nullifier_queue.get_bucket(i).unwrap();
        if let Some(bucket) = bucket {
            if bucket.sequence_number.is_none() {
                let account = crate::v2::queue_data::Account {
                    hash: bucket.value_bytes(),
                    index: i,
                };
                let account_data = AccountData {
                    account,
                    proof: Vec::new(), // This will be filled in during FetchProofs stage
                    leaf_index: 0,     // This will be filled in during FetchProofs stage
                    root_seq: 0,       // This will be filled in during FetchProofs stage
                };
                accounts_to_nullify.push(account_data);
            }
        }
    }
    Ok(accounts_to_nullify)
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
