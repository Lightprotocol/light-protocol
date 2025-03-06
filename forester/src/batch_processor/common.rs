use std::sync::Arc;

use forester_utils::forester_epoch::TreeType;
use light_batched_merkle_tree::{
    batch::{Batch, BatchState},
    merkle_tree::BatchedMerkleTreeAccount,
    queue::BatchedQueueAccount,
};
use light_client::{indexer::Indexer, rpc::RpcConnection, rpc_pool::SolanaRpcPool};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use tokio::sync::Mutex;
use tracing::{debug, info, log::error};

use super::{address, error::Result, state, BatchProcessError};
use crate::indexer_type::IndexerType;

#[derive(Debug)]
pub struct BatchContext<R: RpcConnection, I: Indexer<R>> {
    pub rpc_pool: Arc<SolanaRpcPool<R>>,
    pub indexer: Arc<Mutex<I>>,
    pub authority: Keypair,
    pub derivation: Pubkey,
    pub epoch: u64,
    pub merkle_tree: Pubkey,
    pub output_queue: Pubkey,
}

#[derive(Debug)]
pub enum BatchReadyState {
    NotReady,
    ReadyForAppend,
    ReadyForNullify,
}

#[derive(Debug)]
pub struct BatchProcessor<R: RpcConnection, I: Indexer<R> + IndexerType<R>> {
    context: BatchContext<R, I>,
    tree_type: TreeType,
}

impl<R: RpcConnection, I: Indexer<R> + IndexerType<R>> BatchProcessor<R, I> {
    pub fn new(context: BatchContext<R, I>, tree_type: TreeType) -> Self {
        Self { context, tree_type }
    }

    pub async fn process(&self) -> Result<usize> {
        info!(
            "Starting batch processing for tree type: {:?}",
            self.tree_type
        );
        let state = self.verify_batch_ready().await;
        debug!("Batch ready state: {:?}", state);

        match state {
            BatchReadyState::ReadyForAppend => match self.tree_type {
                TreeType::BatchedAddress => address::process_batch(&self.context).await,
                TreeType::BatchedState => {
                    info!(
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
                    Err(BatchProcessError::UnsupportedTreeType(self.tree_type))
                }
            },
            BatchReadyState::ReadyForNullify => {
                info!(
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
                debug!(
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
        let output_ready = if self.tree_type == TreeType::BatchedState {
            self.verify_output_queue_batch_ready(&mut rpc).await
        } else {
            false
        };

        debug!(
            "self.tree_type: {}, input_ready: {}, output_ready: {}",
            self.tree_type, input_ready, output_ready
        );

        if self.tree_type == TreeType::BatchedAddress {
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

                debug!(
                    "Input queue fill: {:.2}, Output queue fill: {:.2}",
                    input_fill, output_fill
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
        state::perform_append(&self.context, &mut rpc).await?;
        Ok(zkp_batch_size)
    }

    /// Process state nullify operation
    async fn process_state_nullify(&self) -> Result<usize> {
        let mut rpc = self.context.rpc_pool.get_connection().await?;
        let (inserted_zkps_count, zkp_batch_size) = self.get_num_inserted_zkps(&mut rpc).await?;
        debug!(
            "ZKP batch size: {}, inserted ZKPs count: {}",
            zkp_batch_size, inserted_zkps_count
        );
        state::perform_nullify(&self.context, &mut rpc).await?;
        Ok(zkp_batch_size)
    }

    /// Get the number of inserted ZKPs and the ZKP batch size
    async fn get_num_inserted_zkps(&self, rpc: &mut R) -> Result<(u64, usize)> {
        let (num_inserted_zkps, zkp_batch_size) = {
            let mut output_queue_account =
                rpc.get_account(self.context.output_queue).await?.unwrap();
            let output_queue =
                BatchedQueueAccount::output_from_bytes(output_queue_account.data.as_mut_slice())
                    .map_err(|e| BatchProcessError::QueueParsing(e.to_string()))?;

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
            TreeType::BatchedAddress => BatchedMerkleTreeAccount::address_from_bytes(
                account.data.as_mut_slice(),
                &self.context.merkle_tree.into(),
            ),
            TreeType::BatchedState => BatchedMerkleTreeAccount::state_from_bytes(
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
            TreeType::BatchedState => {
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
