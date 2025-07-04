use std::{future::Future, sync::Arc, time::Duration};

use borsh::BorshSerialize;
use forester_utils::rpc_pool::SolanaRpcPool;
pub use forester_utils::{ParsedMerkleTreeData, ParsedQueueData};
use futures::{pin_mut, stream::StreamExt, Stream};
use light_batched_merkle_tree::{
    batch::BatchState, merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_client::rpc::Rpc;
use light_compressed_account::TreeType;
use light_program_test::Indexer;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer};
use tokio::sync::Mutex;
use tracing::{debug, error, info, trace};

use super::{address, state};
use crate::{errors::ForesterError, processor::tx_cache::ProcessedHashCache, Result};

#[derive(Debug)]
pub enum BatchReadyState {
    NotReady,
    AddressReadyForAppend {
        merkle_tree_data: ParsedMerkleTreeData,
    },
    StateReadyForAppend {
        merkle_tree_data: ParsedMerkleTreeData,
        output_queue_data: ParsedQueueData,
    },
    StateReadyForNullify {
        merkle_tree_data: ParsedMerkleTreeData,
    },
}

#[derive(Debug)]
pub struct BatchContext<R: Rpc, I: Indexer> {
    pub rpc_pool: Arc<SolanaRpcPool<R>>,
    pub indexer: Arc<Mutex<I>>,
    pub authority: Keypair,
    pub derivation: Pubkey,
    pub epoch: u64,
    pub merkle_tree: Pubkey,
    pub output_queue: Pubkey,
    pub ixs_per_tx: usize,
    pub prover_url: String,
    pub prover_polling_interval: Duration,
    pub prover_max_wait_time: Duration,
    pub ops_cache: Arc<Mutex<ProcessedHashCache>>,
}

#[derive(Debug)]
pub struct BatchProcessor<R: Rpc, I: Indexer> {
    context: BatchContext<R, I>,
    tree_type: TreeType,
}

/// Processes a stream of batched instruction data into transactions.
pub(crate) async fn process_stream<R, I, S, D, FutC>(
    context: &BatchContext<R, I>,
    stream_creator_future: FutC,
    instruction_builder: impl Fn(&D) -> Instruction,
    tree_type_str: &str,
    operation: Option<&str>,
) -> Result<usize>
where
    R: Rpc,
    I: Indexer,
    S: Stream<Item = Result<Vec<D>>> + Send,
    D: BorshSerialize,
    FutC: Future<Output = Result<(S, u16)>> + Send,
{
    let start_time = std::time::Instant::now();
    trace!("Executing batched stream processor (hybrid)");

    let (batch_stream, zkp_batch_size) = stream_creator_future.await?;

    if zkp_batch_size == 0 {
        trace!("ZKP batch size is 0, no work to do.");
        return Ok(0);
    }

    pin_mut!(batch_stream);
    let mut total_instructions_processed = 0;
    let mut transactions_sent = 0;

    while let Some(batch_result) = batch_stream.next().await {
        let instruction_batch = batch_result?;

        if instruction_batch.is_empty() {
            continue;
        }

        let instructions: Vec<Instruction> =
            instruction_batch.iter().map(&instruction_builder).collect();

        let tx_start = std::time::Instant::now();
        let signature = send_transaction_batch(context, instructions).await?;
        transactions_sent += 1;
        total_instructions_processed += instruction_batch.len();
        let tx_duration = tx_start.elapsed();

        let operation_suffix = operation
            .map(|op| format!(" operation={}", op))
            .unwrap_or_default();
        info!(
            "V2_TPS_METRIC: transaction_sent tree_type={}{} tree={} tx_num={} signature={} instructions={} tx_duration_ms={} (hybrid)",
            tree_type_str, operation_suffix, context.merkle_tree, transactions_sent, signature, instruction_batch.len(), tx_duration.as_millis()
        );
    }

    if total_instructions_processed == 0 {
        trace!("No instructions were processed from the stream.");
        return Ok(0);
    }

    let total_duration = start_time.elapsed();
    let total_items_processed = total_instructions_processed * zkp_batch_size as usize;
    let tps = if total_duration.as_secs_f64() > 0.0 {
        transactions_sent as f64 / total_duration.as_secs_f64()
    } else {
        0.0
    };
    let ips = if total_duration.as_secs_f64() > 0.0 {
        total_instructions_processed as f64 / total_duration.as_secs_f64()
    } else {
        0.0
    };

    let operation_suffix = operation
        .map(|op| format!(" operation={}", op))
        .unwrap_or_default();
    info!(
        "V2_TPS_METRIC: operation_complete tree_type={}{} tree={} epoch={} zkp_batches={} transactions={} instructions={} duration_ms={} tps={:.2} ips={:.2} items_processed={} (hybrid)", 
        tree_type_str, operation_suffix, context.merkle_tree, context.epoch, total_instructions_processed, transactions_sent, total_instructions_processed,
        total_duration.as_millis(), tps, ips, total_items_processed
    );

    info!(
        "Batched stream processing complete. Processed {} total items.",
        total_items_processed
    );

    Ok(total_items_processed)
}

pub(crate) async fn send_transaction_batch<R: Rpc, I: Indexer>(
    context: &BatchContext<R, I>,
    instructions: Vec<Instruction>,
) -> Result<String> {
    info!(
        "Sending transaction with {} instructions...",
        instructions.len()
    );
    let mut rpc = context.rpc_pool.get_connection().await?;
    let signature = rpc
        .create_and_send_transaction(
            &instructions,
            &context.authority.pubkey(),
            &[&context.authority],
        )
        .await?;
    Ok(signature.to_string())
}

impl<R: Rpc, I: Indexer + 'static> BatchProcessor<R, I> {
    pub fn new(context: BatchContext<R, I>, tree_type: TreeType) -> Self {
        Self { context, tree_type }
    }

    pub async fn process(&self) -> Result<usize> {
        trace!(
            "Starting batch processing for tree type: {:?}",
            self.tree_type
        );
        let state = self.verify_batch_ready().await;

        match state {
            BatchReadyState::AddressReadyForAppend { merkle_tree_data } => {
                trace!(
                    "Processing address append for tree: {}",
                    self.context.merkle_tree
                );

                let batch_hash = format!(
                    "address_batch_{}_{}",
                    self.context.merkle_tree, self.context.epoch
                );
                {
                    let mut cache = self.context.ops_cache.lock().await;
                    if cache.contains(&batch_hash) {
                        debug!("Skipping already processed address batch: {}", batch_hash);
                        return Ok(0);
                    }
                    cache.add(&batch_hash);
                }

                let result = address::process_batch(&self.context, merkle_tree_data).await;

                if let Err(ref e) = result {
                    error!(
                        "Address append failed for tree {}: {:?}",
                        self.context.merkle_tree, e
                    );
                }

                let mut cache = self.context.ops_cache.lock().await;
                cache.cleanup_by_key(&batch_hash);
                trace!("Cache cleaned up for batch: {}", batch_hash);

                result
            }
            BatchReadyState::StateReadyForAppend {
                merkle_tree_data,
                output_queue_data,
            } => {
                trace!(
                    "Process state append for tree: {}",
                    self.context.merkle_tree
                );
                let result = self
                    .process_state_append_hybrid(merkle_tree_data, output_queue_data)
                    .await;
                if let Err(ref e) = result {
                    error!(
                        "State append failed for tree {}: {:?}",
                        self.context.merkle_tree, e
                    );
                }
                result
            }
            BatchReadyState::StateReadyForNullify { merkle_tree_data } => {
                trace!(
                    "Processing batch for nullify, tree: {}",
                    self.context.merkle_tree
                );
                let result = self.process_state_nullify_hybrid(merkle_tree_data).await;
                if let Err(ref e) = result {
                    error!(
                        "State nullify failed for tree {}: {:?}",
                        self.context.merkle_tree, e
                    );
                }
                result
            }
            BatchReadyState::NotReady => {
                trace!(
                    "Batch not ready for processing, tree: {}",
                    self.context.merkle_tree
                );
                Ok(0)
            }
        }
    }

    async fn verify_batch_ready(&self) -> BatchReadyState {
        let rpc = match self.context.rpc_pool.get_connection().await {
            Ok(rpc) => rpc,
            Err(_) => return BatchReadyState::NotReady,
        };

        let merkle_tree_account = rpc
            .get_account(self.context.merkle_tree)
            .await
            .ok()
            .flatten();
        let output_queue_account = if self.tree_type == TreeType::StateV2 {
            rpc.get_account(self.context.output_queue)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        let (merkle_tree_data, input_ready) = if let Some(mut account) = merkle_tree_account {
            match self.parse_merkle_tree_account(&mut account) {
                Ok((data, ready)) => (Some(data), ready),
                Err(_) => (None, false),
            }
        } else {
            (None, false)
        };

        let (output_queue_data, output_ready) = if self.tree_type == TreeType::StateV2 {
            if let Some(mut account) = output_queue_account {
                match self.parse_output_queue_account(&mut account) {
                    Ok((data, ready)) => (Some(data), ready),
                    Err(_) => (None, false),
                }
            } else {
                (None, false)
            }
        } else {
            (None, false)
        };

        trace!(
            "self.tree_type: {}, input_ready: {}, output_ready: {}",
            self.tree_type,
            input_ready,
            output_ready
        );

        if !input_ready && !output_ready {
            info!(
                "QUEUE_METRIC: queue_empty tree_type={} tree={}",
                self.tree_type, self.context.merkle_tree
            );
        } else {
            info!("QUEUE_METRIC: queue_has_elements tree_type={} tree={} input_ready={} output_ready={}",
                self.tree_type, self.context.merkle_tree, input_ready, output_ready);
        }

        if self.tree_type == TreeType::AddressV2 {
            return if input_ready {
                if let Some(mt_data) = merkle_tree_data {
                    BatchReadyState::AddressReadyForAppend {
                        merkle_tree_data: mt_data,
                    }
                } else {
                    BatchReadyState::NotReady
                }
            } else {
                BatchReadyState::NotReady
            };
        }

        // For State tree type, balance appends and nullifies operations
        // based on the queue states
        match (input_ready, output_ready) {
            (true, true) => {
                if let (Some(mt_data), Some(oq_data)) = (merkle_tree_data, output_queue_data) {
                    // If both queues are ready, check their fill levels
                    let input_fill = Self::calculate_completion_from_parsed(
                        mt_data.num_inserted_zkps,
                        mt_data.current_zkp_batch_index,
                    );
                    let output_fill = Self::calculate_completion_from_parsed(
                        oq_data.num_inserted_zkps,
                        oq_data.current_zkp_batch_index,
                    );

                    trace!(
                        "Input queue fill: {:.2}, Output queue fill: {:.2}",
                        input_fill,
                        output_fill
                    );
                    if input_fill > output_fill {
                        BatchReadyState::StateReadyForNullify {
                            merkle_tree_data: mt_data,
                        }
                    } else {
                        BatchReadyState::StateReadyForAppend {
                            merkle_tree_data: mt_data,
                            output_queue_data: oq_data,
                        }
                    }
                } else {
                    BatchReadyState::NotReady
                }
            }
            (true, false) => {
                if let Some(mt_data) = merkle_tree_data {
                    BatchReadyState::StateReadyForNullify {
                        merkle_tree_data: mt_data,
                    }
                } else {
                    BatchReadyState::NotReady
                }
            }
            (false, true) => {
                if let (Some(mt_data), Some(oq_data)) = (merkle_tree_data, output_queue_data) {
                    BatchReadyState::StateReadyForAppend {
                        merkle_tree_data: mt_data,
                        output_queue_data: oq_data,
                    }
                } else {
                    BatchReadyState::NotReady
                }
            }
            (false, false) => BatchReadyState::NotReady,
        }
    }

    async fn process_state_nullify_hybrid(
        &self,
        merkle_tree_data: ParsedMerkleTreeData,
    ) -> Result<usize> {
        let zkp_batch_size = merkle_tree_data.zkp_batch_size as usize;

        let batch_hash = format!(
            "state_nullify_hybrid_{}_{}",
            self.context.merkle_tree, self.context.epoch
        );

        {
            let mut cache = self.context.ops_cache.lock().await;
            if cache.contains(&batch_hash) {
                trace!(
                    "Skipping already processed state nullify batch (hybrid): {}",
                    batch_hash
                );
                return Ok(0);
            }
            cache.add(&batch_hash);
        }

        state::perform_nullify(&self.context, merkle_tree_data).await?;

        trace!(
            "State nullify operation (hybrid) completed for tree: {}",
            self.context.merkle_tree
        );
        let mut cache = self.context.ops_cache.lock().await;
        cache.cleanup_by_key(&batch_hash);
        trace!("Cache cleaned up for batch: {}", batch_hash);

        Ok(zkp_batch_size)
    }

    async fn process_state_append_hybrid(
        &self,
        merkle_tree_data: ParsedMerkleTreeData,
        output_queue_data: ParsedQueueData,
    ) -> Result<usize> {
        let zkp_batch_size = output_queue_data.zkp_batch_size as usize;

        let batch_hash = format!(
            "state_append_hybrid_{}_{}",
            self.context.merkle_tree, self.context.epoch
        );
        {
            let mut cache = self.context.ops_cache.lock().await;
            if cache.contains(&batch_hash) {
                trace!(
                    "Skipping already processed state append batch (hybrid): {}",
                    batch_hash
                );
                return Ok(0);
            }
            cache.add(&batch_hash);
        }
        state::perform_append(&self.context, merkle_tree_data, output_queue_data).await?;
        trace!(
            "State append operation (hybrid) completed for tree: {}",
            self.context.merkle_tree
        );

        let mut cache = self.context.ops_cache.lock().await;
        cache.cleanup_by_key(&batch_hash);

        Ok(zkp_batch_size)
    }

    /// Parse merkle tree account and check if batch is ready
    fn parse_merkle_tree_account(
        &self,
        account: &mut solana_sdk::account::Account,
    ) -> Result<(ParsedMerkleTreeData, bool)> {
        let merkle_tree = match self.tree_type {
            TreeType::AddressV2 => BatchedMerkleTreeAccount::address_from_bytes(
                account.data.as_mut_slice(),
                &self.context.merkle_tree.into(),
            ),
            TreeType::StateV2 => BatchedMerkleTreeAccount::state_from_bytes(
                account.data.as_mut_slice(),
                &self.context.merkle_tree.into(),
            ),
            _ => return Err(ForesterError::InvalidTreeType(self.tree_type).into()),
        }?;

        let batch_index = merkle_tree.queue_batches.pending_batch_index;
        let batch = merkle_tree
            .queue_batches
            .batches
            .get(batch_index as usize)
            .ok_or_else(|| anyhow::anyhow!("Batch not found"))?;

        let num_inserted_zkps = batch.get_num_inserted_zkps();
        let current_zkp_batch_index = batch.get_current_zkp_batch_index();

        let mut leaves_hash_chains = Vec::new();
        for i in num_inserted_zkps..current_zkp_batch_index {
            leaves_hash_chains
                .push(merkle_tree.hash_chain_stores[batch_index as usize][i as usize]);
        }

        let parsed_data = ParsedMerkleTreeData {
            next_index: merkle_tree.next_index,
            current_root: *merkle_tree.root_history.last().unwrap(),
            root_history: merkle_tree.root_history.to_vec(),
            zkp_batch_size: batch.zkp_batch_size as u16,
            pending_batch_index: batch_index as u32,
            num_inserted_zkps,
            current_zkp_batch_index,
            leaves_hash_chains,
        };

        let is_ready = batch.get_state() != BatchState::Inserted
            && batch.get_current_zkp_batch_index() > batch.get_num_inserted_zkps();

        Ok((parsed_data, is_ready))
    }

    /// Parse output queue account and check if batch is ready
    fn parse_output_queue_account(
        &self,
        account: &mut solana_sdk::account::Account,
    ) -> Result<(ParsedQueueData, bool)> {
        let output_queue = BatchedQueueAccount::output_from_bytes(account.data.as_mut_slice())?;

        let batch_index = output_queue.batch_metadata.pending_batch_index;
        let batch = output_queue
            .batch_metadata
            .batches
            .get(batch_index as usize)
            .ok_or_else(|| anyhow::anyhow!("Batch not found"))?;

        let num_inserted_zkps = batch.get_num_inserted_zkps();
        let current_zkp_batch_index = batch.get_current_zkp_batch_index();

        let mut leaves_hash_chains = Vec::new();
        for i in num_inserted_zkps..current_zkp_batch_index {
            leaves_hash_chains
                .push(output_queue.hash_chain_stores[batch_index as usize][i as usize]);
        }

        let parsed_data = ParsedQueueData {
            zkp_batch_size: output_queue.batch_metadata.zkp_batch_size as u16,
            pending_batch_index: batch_index as u32,
            num_inserted_zkps,
            current_zkp_batch_index,
            leaves_hash_chains,
        };

        let is_ready = batch.get_state() != BatchState::Inserted
            && batch.get_current_zkp_batch_index() > batch.get_num_inserted_zkps();

        Ok((parsed_data, is_ready))
    }

    /// Calculate completion percentage from parsed data
    fn calculate_completion_from_parsed(
        num_inserted_zkps: u64,
        current_zkp_batch_index: u64,
    ) -> f64 {
        let total = current_zkp_batch_index;
        if total == 0 {
            return 0.0;
        }
        let remaining = total - num_inserted_zkps;
        remaining as f64 / total as f64
    }
}
