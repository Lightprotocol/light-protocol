use std::{future::Future, sync::Arc, time::Duration};

use borsh::BorshSerialize;
use forester_utils::rpc_pool::SolanaRpcPool;
use futures::{pin_mut, stream::StreamExt, Stream};
use light_batched_merkle_tree::{
    batch::{Batch, BatchState},
    merkle_tree::BatchedMerkleTreeAccount,
    queue::BatchedQueueAccount,
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

/// Processes a stream of instruction data into batches and sends them as transactions.
///
/// # Type Parameters
/// * `R` - RPC client type
/// * `I` - Indexer type
/// * `S` - Stream type yielding instruction data
/// * `D` - Instruction data type (must be BorshSerializable)
/// * `FutC` - Future that creates the stream and returns zkp batch size
///
/// # Arguments
/// * `context` - Batch processing context containing RPC pool, authority, etc.
/// * `stream_creator_future` - Future that creates the instruction stream
/// * `instruction_builder` - Function to convert data to Solana instructions
/// * `tree_type_str` - Tree type identifier for logging
/// * `operation` - Optional operation name for logging
///
/// # Returns
/// Total number of items processed (instructions * zkp_batch_size)
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
    S: Stream<Item = Result<D>> + Send,
    D: BorshSerialize,
    FutC: Future<Output = Result<(S, u16)>> + Send,
{
    let start_time = std::time::Instant::now();
    trace!("Executing generic stream processor");

    let (instruction_stream, zkp_batch_size) = stream_creator_future.await?;

    if zkp_batch_size == 0 {
        trace!("ZKP batch size is 0, no work to do.");
        return Ok(0);
    }

    pin_mut!(instruction_stream);
    let mut instruction_buffer: Vec<D> = Vec::new();
    let mut total_instructions_processed = 0;
    let mut transactions_sent = 0;

    while let Some(result) = instruction_stream.next().await {
        let data = result?;
        instruction_buffer.push(data);
        total_instructions_processed += 1;

        if instruction_buffer.len() >= context.ixs_per_tx {
            let instructions = instruction_buffer
                .iter()
                .map(&instruction_builder)
                .collect();

            let tx_start = std::time::Instant::now();
            let signature = send_transaction_batch(context, instructions).await?;
            transactions_sent += 1;
            let tx_duration = tx_start.elapsed();

            let operation_suffix = operation
                .map(|op| format!(" operation={}", op))
                .unwrap_or_default();
            info!(
                "V2_TPS_METRIC: transaction_sent tree_type={}{} tree={} tx_num={} signature={} instructions={} tx_duration_ms={}",
                tree_type_str, operation_suffix, context.merkle_tree, transactions_sent, signature, instruction_buffer.len(), tx_duration.as_millis()
            );

            instruction_buffer.clear();
        }
    }

    if !instruction_buffer.is_empty() {
        let instructions = instruction_buffer
            .iter()
            .map(&instruction_builder)
            .collect();

        let tx_start = std::time::Instant::now();
        let signature = send_transaction_batch(context, instructions).await?;
        transactions_sent += 1;
        let tx_duration = tx_start.elapsed();

        let operation_suffix = operation
            .map(|op| format!(" operation={}", op))
            .unwrap_or_default();
        info!(
            "V2_TPS_METRIC: transaction_sent tree_type={}{} tree={} tx_num={} signature={} instructions={} tx_duration_ms={}",
            tree_type_str, operation_suffix, context.merkle_tree, transactions_sent, signature, instruction_buffer.len(), tx_duration.as_millis()
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
        "V2_TPS_METRIC: operation_complete tree_type={}{} tree={} epoch={} zkp_batches={} transactions={} instructions={} duration_ms={} tps={:.2} ips={:.2} items_processed={}", tree_type_str, operation_suffix, context.merkle_tree, context.epoch, total_instructions_processed, transactions_sent, total_instructions_processed,
        total_duration.as_millis(), tps, ips, total_items_processed
    );

    info!(
        "Stream processing complete. Processed {} total items.",
        total_items_processed
    );

    Ok(total_items_processed)
}

async fn send_transaction_batch<R: Rpc, I: Indexer>(
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

#[derive(Debug)]
pub enum BatchReadyState {
    NotReady,
    ReadyForAppend,
    ReadyForNullify,
}

#[derive(Debug)]
pub struct BatchProcessor<R: Rpc, I: Indexer> {
    context: BatchContext<R, I>,
    tree_type: TreeType,
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
            BatchReadyState::ReadyForAppend => match self.tree_type {
                TreeType::AddressV2 => {
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

                    let result = address::process_batch(&self.context).await;

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
                TreeType::StateV2 => {
                    trace!(
                        "Process state append for tree: {}",
                        self.context.merkle_tree
                    );
                    let result = self.process_state_append().await;
                    if let Err(ref e) = result {
                        error!(
                            "State append failed for tree {}: {:?}",
                            self.context.merkle_tree, e
                        );
                    }
                    result
                }
                _ => {
                    error!("Unsupported tree type for append: {:?}", self.tree_type);
                    Err(ForesterError::InvalidTreeType(self.tree_type).into())
                }
            },
            BatchReadyState::ReadyForNullify => {
                trace!(
                    "Processing batch for nullify, tree: {}",
                    self.context.merkle_tree
                );
                let result = self.process_state_nullify().await;
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
        let mut rpc = match self.context.rpc_pool.get_connection().await {
            Ok(rpc) => rpc,
            Err(_) => return BatchReadyState::NotReady,
        };

        let input_ready = self.verify_input_queue_batch_ready(&mut rpc).await;
        let output_ready = if self.tree_type == TreeType::StateV2 {
            self.verify_output_queue_batch_ready(&mut rpc).await
        } else {
            false
        };

        trace!(
            "self.tree_type: {}, input_ready: {}, output_ready: {}",
            self.tree_type,
            input_ready,
            output_ready
        );

        // Log queue metrics
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
                BatchReadyState::ReadyForAppend
            } else {
                BatchReadyState::NotReady
            };
        }

        // For State tree type, balance append and nullify operations
        // based on the queue states
        match (input_ready, output_ready) {
            (true, true) => {
                // If both queues are ready, check their fill levels
                let input_fill = self.get_input_queue_completion(&mut rpc).await;
                let output_fill = self.get_output_queue_completion(&mut rpc).await;

                trace!(
                    "Input queue fill: {:.2}, Output queue fill: {:.2}",
                    input_fill,
                    output_fill
                );
                if input_fill > output_fill {
                    BatchReadyState::ReadyForNullify
                } else {
                    BatchReadyState::ReadyForAppend
                }
            }
            (true, false) => BatchReadyState::ReadyForNullify,
            (false, true) => BatchReadyState::ReadyForAppend,
            (false, false) => BatchReadyState::NotReady,
        }
    }
    /// Get the completion percentage of the input queue
    async fn get_input_queue_completion(&self, rpc: &mut R) -> f64 {
        let mut account = match rpc.get_account(self.context.merkle_tree).await {
            Ok(Some(account)) => account,
            _ => return 0.0,
        };

        Self::calculate_completion_from_tree(account.data.as_mut_slice())
    }

    /// Get the completion percentage of the output queue
    async fn get_output_queue_completion(&self, rpc: &mut R) -> f64 {
        let mut account = match rpc.get_account(self.context.output_queue).await {
            Ok(Some(account)) => account,
            _ => return 0.0,
        };

        Self::calculate_completion_from_queue(account.data.as_mut_slice())
    }

    /// Calculate completion percentage from a merkle tree account
    fn calculate_completion_from_tree(data: &mut [u8]) -> f64 {
        let tree = match BatchedMerkleTreeAccount::state_from_bytes(data, &Pubkey::default().into())
        {
            Ok(tree) => tree,
            Err(_) => return 0.0,
        };

        let batch_index = tree.queue_batches.pending_batch_index;
        match tree.queue_batches.batches.get(batch_index as usize) {
            Some(batch) => Self::calculate_completion(batch),
            None => 0.0,
        }
    }

    /// Calculate completion percentage from a queue account
    fn calculate_completion_from_queue(data: &mut [u8]) -> f64 {
        let queue = match BatchedQueueAccount::output_from_bytes(data) {
            Ok(queue) => queue,
            Err(_) => return 0.0,
        };

        let batch_index = queue.batch_metadata.pending_batch_index;
        match queue.batch_metadata.batches.get(batch_index as usize) {
            Some(batch) => Self::calculate_completion(batch),
            None => 0.0,
        }
    }

    /// Calculate completion percentage for a batch
    fn calculate_completion(batch: &Batch) -> f64 {
        let total = batch.get_num_zkp_batches();
        if total == 0 {
            return 0.0;
        }

        let remaining = total - batch.get_num_inserted_zkps();
        remaining as f64 / total as f64
    }

    /// Process state append operation
    async fn process_state_append(&self) -> Result<usize> {
        let mut rpc = self.context.rpc_pool.get_connection().await?;
        let (_, zkp_batch_size) = self.get_num_inserted_zkps(&mut rpc).await?;

        let batch_hash = format!(
            "state_append_{}_{}",
            self.context.merkle_tree, self.context.epoch
        );
        {
            let mut cache = self.context.ops_cache.lock().await;
            if cache.contains(&batch_hash) {
                trace!(
                    "Skipping already processed state append batch: {}",
                    batch_hash
                );
                return Ok(0);
            }
            cache.add(&batch_hash);
        }
        state::perform_append(&self.context).await?;
        trace!(
            "State append operation completed for tree: {}",
            self.context.merkle_tree
        );

        let mut cache = self.context.ops_cache.lock().await;
        cache.cleanup_by_key(&batch_hash);

        Ok(zkp_batch_size)
    }

    /// Process state nullify operation
    async fn process_state_nullify(&self) -> Result<usize> {
        let mut rpc = self.context.rpc_pool.get_connection().await?;
        let (inserted_zkps_count, zkp_batch_size) = self.get_num_inserted_zkps(&mut rpc).await?;
        trace!(
            "ZKP batch size: {}, inserted ZKPs count: {}",
            zkp_batch_size,
            inserted_zkps_count
        );

        let batch_hash = format!(
            "state_nullify_{}_{}",
            self.context.merkle_tree, self.context.epoch
        );

        {
            let mut cache = self.context.ops_cache.lock().await;
            if cache.contains(&batch_hash) {
                trace!(
                    "Skipping already processed state nullify batch: {}",
                    batch_hash
                );
                return Ok(0);
            }
            cache.add(&batch_hash);
        }

        state::perform_nullify(&self.context).await?;

        trace!(
            "State nullify operation completed for tree: {}",
            self.context.merkle_tree
        );
        let mut cache = self.context.ops_cache.lock().await;
        cache.cleanup_by_key(&batch_hash);
        trace!("Cache cleaned up for batch: {}", batch_hash);

        Ok(zkp_batch_size)
    }

    /// Get the number of inserted ZKPs and the ZKP batch size
    async fn get_num_inserted_zkps(&self, rpc: &mut R) -> Result<(u64, usize)> {
        let (num_inserted_zkps, zkp_batch_size) = {
            let mut output_queue_account =
                rpc.get_account(self.context.output_queue).await?.unwrap();
            let output_queue =
                BatchedQueueAccount::output_from_bytes(output_queue_account.data.as_mut_slice())?;

            let batch_index = output_queue.batch_metadata.pending_batch_index;
            let zkp_batch_size = output_queue.batch_metadata.zkp_batch_size;

            (
                output_queue.batch_metadata.batches[batch_index as usize].get_num_inserted_zkps(),
                zkp_batch_size as usize,
            )
        };
        Ok((num_inserted_zkps, zkp_batch_size))
    }

    /// Verify if the input queue batch is ready for processing
    async fn verify_input_queue_batch_ready(&self, rpc: &mut R) -> bool {
        let mut account = match rpc.get_account(self.context.merkle_tree).await {
            Ok(Some(account)) => account,
            _ => return false,
        };

        let merkle_tree = match self.tree_type {
            TreeType::AddressV2 => BatchedMerkleTreeAccount::address_from_bytes(
                account.data.as_mut_slice(),
                &self.context.merkle_tree.into(),
            ),
            TreeType::StateV2 => BatchedMerkleTreeAccount::state_from_bytes(
                account.data.as_mut_slice(),
                &self.context.merkle_tree.into(),
            ),
            _ => return false,
        };

        if let Ok(tree) = merkle_tree {
            let batch_index = tree.queue_batches.pending_batch_index;
            let full_batch = tree
                .queue_batches
                .batches
                .get(batch_index as usize)
                .unwrap();

            full_batch.get_state() != BatchState::Inserted
                && full_batch.get_current_zkp_batch_index() > full_batch.get_num_inserted_zkps()
        } else {
            false
        }
    }

    /// Verify if the output queue batch is ready for processing
    async fn verify_output_queue_batch_ready(&self, rpc: &mut R) -> bool {
        let mut account = match rpc.get_account(self.context.output_queue).await {
            Ok(Some(account)) => account,
            _ => return false,
        };

        let output_queue = match self.tree_type {
            TreeType::StateV2 => {
                BatchedQueueAccount::output_from_bytes(account.data.as_mut_slice())
            }
            _ => return false,
        };

        if let Ok(queue) = output_queue {
            let batch_index = queue.batch_metadata.pending_batch_index;
            let full_batch = queue
                .batch_metadata
                .batches
                .get(batch_index as usize)
                .unwrap();

            full_batch.get_state() != BatchState::Inserted
                && full_batch.get_current_zkp_batch_index() > full_batch.get_num_inserted_zkps()
        } else {
            false
        }
    }
}
