use std::sync::Arc;

use forester_utils::{forester_epoch::TreeType, indexer::Indexer};
use light_batched_merkle_tree::{
    batch::BatchState, merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_client::{rpc::RpcConnection, rpc_pool::SolanaRpcPool};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use tokio::sync::Mutex;
use tracing::info;
use light_batched_merkle_tree::batch::Batch;
use super::{address, error::Result, state, BatchProcessError};

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
pub struct BatchProcessor<R: RpcConnection, I: Indexer<R>> {
    context: BatchContext<R, I>,
    tree_type: TreeType,
}

impl<R: RpcConnection, I: Indexer<R>> BatchProcessor<R, I> {
    pub fn new(context: BatchContext<R, I>, tree_type: TreeType) -> Self {
        Self { context, tree_type }
    }

    pub async fn process(&self) -> Result<usize> {
        match self.verify_batch_ready().await {
            BatchReadyState::ReadyForAppend => match self.tree_type {
                TreeType::BatchedAddress => address::process_batch(&self.context).await,
                TreeType::BatchedState => self.process_state_append().await,
                _ => Err(BatchProcessError::UnsupportedTreeType(self.tree_type)),
            },
            BatchReadyState::ReadyForNullify => self.process_state_nullify().await,
            BatchReadyState::NotReady => Ok(0),
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

        if self.tree_type == TreeType::BatchedAddress {
            return if input_ready {
                BatchReadyState::ReadyForAppend
            } else {
                BatchReadyState::NotReady
            };
        }

        // For State tree type, we need to balance between append and nullify
        // operations based on the queue states
        match (input_ready, output_ready) {
            (true, true) => {
                // If both queues are ready, check their fill levels
                let input_fill = self.get_input_queue_completion(&mut rpc).await;
                let output_fill = self.get_output_queue_completion(&mut rpc).await;

                info!(
                    "Input queue fill: {:.2}, Output queue fill: {:.2}",
                    input_fill, output_fill
                );
                // Prioritize the queue that is more full
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
    async fn get_input_queue_completion(&self, rpc: &mut R) -> f64 {
        let mut account = match rpc.get_account(self.context.merkle_tree).await {
            Ok(Some(account)) => account,
            _ => return 0.0,
        };

        Self::calculate_completion_from_tree(account.data.as_mut_slice())
    }

    async fn get_output_queue_completion(&self, rpc: &mut R) -> f64 {
        let mut account = match rpc.get_account(self.context.output_queue).await {
            Ok(Some(account)) => account,
            _ => return 0.0,
        };

        Self::calculate_completion_from_queue(account.data.as_mut_slice())
    }

    fn calculate_completion_from_tree(data: &mut [u8]) -> f64 {
        let tree = match BatchedMerkleTreeAccount::state_tree_from_bytes_mut(data) {
            Ok(tree) => tree,
            Err(_) => return 0.0,
        };

        let batch_index = tree.get_metadata().queue_metadata.next_full_batch_index;
        match tree.batches.get(batch_index as usize) {
            Some(batch) => Self::calculate_completion(batch),
            None => 0.0,
        }
    }

    fn calculate_completion_from_queue(data: &mut [u8]) -> f64 {
        let queue = match BatchedQueueAccount::output_queue_from_bytes_mut(data) {
            Ok(queue) => queue,
            Err(_) => return 0.0,
        };

        let batch_index = queue.get_metadata().batch_metadata.next_full_batch_index;
        match queue.batches.get(batch_index as usize) {
            Some(batch) => Self::calculate_completion(batch),
            None => 0.0,
        }
    }

    fn calculate_completion(batch: &Batch) -> f64 {
        let total = batch.get_num_zkp_batches();
        if total == 0 {
            return 0.0;
        }

        let remaining = total - batch.get_num_inserted_zkps();
        remaining as f64 / total as f64
    }

    async fn process_state_append(&self) -> Result<usize> {
        let mut rpc = self.context.rpc_pool.get_connection().await?;
        let (num_inserted_zkps, zkp_batch_size) = self.get_num_inserted_zkps(&mut rpc).await?;
        state::perform_append(&self.context, &mut rpc, num_inserted_zkps).await?;
        Ok(zkp_batch_size)
    }

    async fn process_state_nullify(&self) -> Result<usize> {
        let mut rpc = self.context.rpc_pool.get_connection().await?;
        let (_, zkp_batch_size) = self.get_num_inserted_zkps(&mut rpc).await?;
        state::perform_nullify(&self.context, &mut rpc).await?;
        Ok(zkp_batch_size)
    }

    async fn get_num_inserted_zkps(&self, rpc: &mut R) -> Result<(u64, usize)> {
        let (num_inserted_zkps, zkp_batch_size) = {
            let mut output_queue_account =
                rpc.get_account(self.context.output_queue).await?.unwrap();
            let output_queue = BatchedQueueAccount::output_queue_from_bytes_mut(
                output_queue_account.data.as_mut_slice(),
            )
            .map_err(|e| BatchProcessError::QueueParsing(e.to_string()))?;

            let batch_index = output_queue
                .get_metadata()
                .batch_metadata
                .next_full_batch_index;
            let zkp_batch_size = output_queue.get_metadata().batch_metadata.zkp_batch_size;

            (
                output_queue.batches[batch_index as usize].get_num_inserted_zkps(),
                zkp_batch_size as usize,
            )
        };
        Ok((num_inserted_zkps, zkp_batch_size))
    }

    async fn verify_input_queue_batch_ready(&self, rpc: &mut R) -> bool {
        let mut account = match rpc.get_account(self.context.merkle_tree).await {
            Ok(Some(account)) => account,
            _ => return false,
        };

        let merkle_tree = match self.tree_type {
            TreeType::BatchedAddress => {
                BatchedMerkleTreeAccount::address_tree_from_bytes_mut(account.data.as_mut_slice())
            }
            TreeType::BatchedState => {
                BatchedMerkleTreeAccount::state_tree_from_bytes_mut(account.data.as_mut_slice())
            }
            _ => return false,
        };

        if let Ok(tree) = merkle_tree {
            let batch_index = tree.get_metadata().queue_metadata.next_full_batch_index;
            let full_batch = tree.batches.get(batch_index as usize).unwrap();

            full_batch.get_state() != BatchState::Inserted
                && full_batch.get_current_zkp_batch_index() > full_batch.get_num_inserted_zkps()
        } else {
            false
        }
    }

    async fn verify_output_queue_batch_ready(&self, rpc: &mut R) -> bool {
        let mut account = match rpc.get_account(self.context.output_queue).await {
            Ok(Some(account)) => account,
            _ => return false,
        };

        let output_queue = match self.tree_type {
            TreeType::BatchedState => {
                BatchedQueueAccount::output_queue_from_bytes_mut(account.data.as_mut_slice())
            }
            _ => return false,
        };

        if let Ok(queue) = output_queue {
            let batch_index = queue.get_metadata().batch_metadata.next_full_batch_index;
            let full_batch = queue.batches.get(batch_index as usize).unwrap();

            full_batch.get_state() != BatchState::Inserted
                && full_batch.get_current_zkp_batch_index() > full_batch.get_num_inserted_zkps()
        } else {
            false
        }
    }
}
