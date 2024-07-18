use crate::errors::ForesterError;
use crate::nullifier::state::pipeline::StatePipelineStage;
use crate::nullifier::{BackpressureControl, ForesterQueueAccountData, PipelineContext};
use crate::operations::fetch_state_queue_data;
use crate::tree_sync::TreeData;
use crate::{ForesterConfig, RpcPool};
use account_compression::utils::constants::STATE_MERKLE_TREE_CHANGELOG;
use light_registry::sdk::{create_nullify_instruction, CreateNullifyInstructionInputs};
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use log::{debug, error, info, warn};
use rand::seq::SliceRandom;
use rand::thread_rng;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};

pub struct StateProcessor<T: Indexer<R>, R: RpcConnection> {
    pub input: mpsc::Receiver<StatePipelineStage<T, R>>,
    pub output: mpsc::Sender<StatePipelineStage<T, R>>,
    pub backpressure: BackpressureControl,
    pub shutdown: Arc<AtomicBool>,
    pub close_output: mpsc::Receiver<()>,
    pub state_queue: Arc<Mutex<Vec<ForesterQueueAccountData>>>,
}

impl<T: Indexer<R>, R: RpcConnection> StateProcessor<T, R> {
    pub(crate) async fn process(&mut self) {
        debug!("Starting StateProcessor process");
        let mut consecutive_errors = 0;
        loop {
            tokio::select! {
                Some(item) = self.input.recv() => {
                    debug!("Received item in StateProcessor");
                    let _permit = self.backpressure.acquire().await;
                    let result = match item {
                        StatePipelineStage::FetchStateQueueData(context) => {
                            info!("Processing FetchStateQueueData stage");
                            self.fetch_state_queue_data(&context).await
                        }
                        StatePipelineStage::FetchProofs(context, queue_data) => {
                            info!("Processing FetchProofs stage");
                            self.fetch_proofs(context, queue_data).await
                        }
                        StatePipelineStage::NullifyStateBatch(context, account_data) => {
                            info!("Processing NullifyStateBatch stage");
                            self.nullify_state_batch(context, account_data).await
                        }
                        StatePipelineStage::Complete => {
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
                            warn!("Error in StateProcessor: {:?}", e);
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
                debug!("Shutdown signal received, stopping StateProcessor");
                break;
            }
        }
        debug!("StateProcessor process completed");
    }

    async fn fetch_state_queue_data(
        &self,
        context: &PipelineContext<T, R>,
    ) -> Result<Option<StatePipelineStage<T, R>>, ForesterError> {
        info!("Starting to fetch state queue data");
        let mut state_queue = self.state_queue.lock().await;

        if !state_queue.is_empty() {
            let batch_size = state_queue.len().min(context.config.batch_size);
            let batch: Vec<ForesterQueueAccountData> = state_queue.drain(..batch_size).collect();
            return Ok(Some(StatePipelineStage::FetchProofs(
                context.clone(),
                batch,
            )));
        }

        let mut queue_data = {
            let rpc = context.rpc_pool.get_connection().await;
            let mut queue_data = fetch_state_queue_data(rpc, context.tree_data).await?;
            let mut rng = thread_rng();
            queue_data.data.shuffle(&mut rng);
            queue_data
        };

        info!("Fetched state queue data len: {:?}", queue_data.data.len());
        if queue_data.data.is_empty() {
            info!("State queue is empty");
            Ok(Some(StatePipelineStage::Complete))
        } else {
            let batch_size = queue_data.data.len().min(context.config.batch_size);
            let batch: Vec<ForesterQueueAccountData> =
                queue_data.data.drain(..batch_size).collect();
            *state_queue = queue_data.data;
            Ok(Some(StatePipelineStage::FetchProofs(
                context.clone(),
                batch,
            )))
        }
    }

    async fn fetch_proofs(
        &self,
        context: PipelineContext<T, R>,
        queue_data: Vec<ForesterQueueAccountData>,
    ) -> Result<Option<StatePipelineStage<T, R>>, ForesterError> {
        let indexer = &context.indexer;

        let states = queue_data
            .iter()
            .map(|account| account.account.hash_string())
            .collect();

        let proofs = match indexer
            .lock()
            .await
            .get_multiple_compressed_account_proofs(states)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                warn!("Error fetching proofs: {:?}", e);
                return Ok(Some(StatePipelineStage::Complete));
            }
        };

        if proofs.is_empty() {
            return Ok(Some(StatePipelineStage::FetchStateQueueData(context)));
        }

        let account_data = queue_data
            .into_iter()
            .zip(proofs.into_iter())
            .map(|(account, proof)| ForesterQueueAccountData {
                account: account.account,
                proof: proof.proof,
                leaf_index: proof.leaf_index as u64,
                root_seq: proof.root_seq,
            })
            .collect();

        Ok(Some(StatePipelineStage::NullifyStateBatch(
            context,
            account_data,
        )))
    }

    async fn nullify_state_batch(
        &self,
        context: PipelineContext<T, R>,
        account_data_batch: Vec<ForesterQueueAccountData>,
    ) -> Result<Option<StatePipelineStage<T, R>>, ForesterError> {
        let indexer = &context.indexer;

        let (tx, mut rx) = mpsc::channel(account_data_batch.len());

        for account_data in account_data_batch {
            let tx = tx.clone();
            let context = context.clone();

            tokio::spawn(async move {
                let mut retry_count = 0;
                while retry_count < context.config.max_retries {
                    match nullify_state(
                        context.rpc_pool.clone(),
                        context.config.clone(),
                        &context.config.payer_keypair,
                        context.tree_data,
                        account_data.account.index as u16,
                        account_data.leaf_index,
                        account_data.proof.clone(),
                        account_data.root_seq,
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
                                "Error nullifying state for account {:?}: {:?}",
                                account_data.account.hash_string(),
                                e
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
                debug!("State nullified: {:?}", account_data.account.hash_string());
                indexer.lock().await.account_nullified(
                    context.tree_data.tree_pubkey,
                    &account_data.account.hash_string(),
                );
            } else {
                warn!(
                    "Failed to nullify state for account: {:?}",
                    account_data.account.hash_string()
                );
            }
        }

        let state_queue = self.state_queue.lock().await;
        if state_queue.is_empty() {
            Ok(Some(StatePipelineStage::FetchStateQueueData(context)))
        } else {
            Ok(Some(StatePipelineStage::FetchProofs(context, Vec::new())))
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn nullify_state<R: RpcConnection>(
    rpc_pool: RpcPool<R>,
    config: Arc<ForesterConfig>,
    payer: &Keypair,
    tree_data: TreeData,
    leaves_queue_index: u16,
    leaf_index: u64,
    proof: Vec<[u8; 32]>,
    root_seq: u64,
) -> Result<bool, ForesterError> {
    let start = Instant::now();
    debug!("root_seq: {:?}", root_seq);

    let change_log_index = root_seq % STATE_MERKLE_TREE_CHANGELOG;
    debug!("change_log_index: {:?}", change_log_index);

    let ix = create_nullify_instruction(CreateNullifyInstructionInputs {
        nullifier_queue: tree_data.queue_pubkey,
        merkle_tree: tree_data.tree_pubkey,
        change_log_indices: vec![change_log_index],
        leaves_queue_indices: vec![leaves_queue_index],
        indices: vec![leaf_index],
        proofs: vec![proof],
        authority: payer.pubkey(),
        derivation: Pubkey::from_str(&config.external_services.derivation).unwrap(),
    });

    let instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(config.cu_limit),
        ix,
    ];

    let latest_blockhash = {
        let rpc = rpc_pool.get_connection().await;
        let mut rpc_guard = rpc.lock().await;
        rpc_guard.get_latest_blockhash().await?
    };

    let blockhash_time = start.elapsed();
    info!("Time to get blockhash: {:?}", blockhash_time);

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &[payer],
        latest_blockhash,
    );

    let (signature, confirmed) = {
        let rpc = rpc_pool.get_connection().await;
        let mut rpc_guard = rpc.lock().await;
        let signature = rpc_guard.process_transaction(transaction).await?;
        let confirmed = rpc_guard.confirm_transaction(signature).await?;
        (signature, confirmed)
    };

    let total_time = start.elapsed();
    info!("Total time for transaction: {:?}", total_time);

    info!("Processed: {:?}", signature);
    info!("Confirmed: {:?} {}", signature, confirmed);

    Ok(confirmed)
}
