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

    async fn verify_batch_ready(&self) -> BatchReadyState {
        let mut rpc = match self.context.rpc_pool.get_connection().await {
            Ok(rpc) => rpc,
            Err(_) => return BatchReadyState::NotReady,
        };

        if self.tree_type == TreeType::BatchedAddress {
            return if self.verify_input_queue_batch_ready(&mut rpc).await {
                BatchReadyState::ReadyForAppend
            } else {
                BatchReadyState::NotReady
            };
        }

        if self.verify_input_queue_batch_ready(&mut rpc).await {
            BatchReadyState::ReadyForNullify
        } else if self.verify_output_queue_batch_ready(&mut rpc).await {
            BatchReadyState::ReadyForAppend
        } else {
            BatchReadyState::NotReady
        }
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
        info!("verify_output_queue_batch_ready");
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

        info!("output_queue: {:?}", output_queue);

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
