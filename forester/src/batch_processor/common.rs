use std::sync::Arc;

use forester_utils::{forester_epoch::TreeType, indexer::Indexer};
use light_batched_merkle_tree::{
    batch::BatchState, merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_client::{rpc::RpcConnection, rpc_pool::SolanaRpcPool};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use tokio::sync::Mutex;
use tracing::{debug, info, instrument};

use super::{address, error, error::Result, state};

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
pub struct BatchProcessor<R: RpcConnection, I: Indexer<R>> {
    context: BatchContext<R, I>,
    tree_type: TreeType,
}

impl<R: RpcConnection, I: Indexer<R>> BatchProcessor<R, I> {
    pub fn new(context: BatchContext<R, I>, tree_type: TreeType) -> Self {
        Self { context, tree_type }
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn process(&self) -> Result<usize> {
        if !self.verify_batch_ready().await {
            debug!("Batch is not ready for processing");
            return Ok(0);
        }

        match self.tree_type {
            TreeType::BatchedAddress => {
                info!("Processing address batch");
                address::process_batch(&self.context).await
            }
            TreeType::BatchedState => {
                info!("Processing state batch");
                state::process_batch(&self.context).await
            }
            _ => Err(error::BatchProcessError::UnsupportedTreeType(
                self.tree_type,
            )),
        }
    }

    async fn verify_batch_ready(&self) -> bool {
        let mut rpc = match self.context.rpc_pool.get_connection().await {
            Ok(rpc) => rpc,
            Err(_) => return false,
        };

        if self.tree_type == TreeType::BatchedAddress {
            return self.verify_input_queue_batch_ready(&mut rpc).await;
        }

        let input_queue_ready = self.verify_input_queue_batch_ready(&mut rpc).await;
        let output_queue_ready = self.verify_output_queue_batch_ready(&mut rpc).await;

        input_queue_ready && output_queue_ready
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
