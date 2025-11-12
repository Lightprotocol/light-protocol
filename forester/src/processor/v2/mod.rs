pub mod coordinator;

mod address;
mod common;

use forester_utils::batch_parsing::parse_merkle_tree_batch;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_client::rpc::Rpc;
use light_compressed_account::TreeType;
use tracing::{debug, instrument};

use crate::Result;
use coordinator::StateTreeCoordinator;

#[instrument(
    level = "debug",
    fields(
        epoch = context.epoch,
        tree = %context.merkle_tree,
        tree_type = ?tree_type
    ),
    skip(context)
)]
pub async fn process_batched_operations<R: Rpc>(
    context: BatchContext<R>,
    tree_type: TreeType,
) -> Result<usize> {
    match tree_type {
        TreeType::StateV2 => {
            let rpc = context.rpc_pool.get_connection().await?;
            let mut account = rpc.get_account(context.merkle_tree).await?.ok_or_else(|| {
                anyhow::anyhow!("Merkle tree account not found")
            })?;

            let tree_data = BatchedMerkleTreeAccount::state_from_bytes(
                account.data.as_mut_slice(),
                &context.merkle_tree.into(),
            )?;

            let initial_root = tree_data.root_history.last().copied().ok_or_else(|| {
                anyhow::anyhow!("No root in tree history")
            })?;

            drop(rpc);

            debug!("Processing StateV2 tree with StateTreeCoordinator");
            let mut coordinator = StateTreeCoordinator::new(context, initial_root).await;
            coordinator.process().await
        }
        TreeType::AddressV2 => {
            let rpc = context.rpc_pool.get_connection().await?;
            let mut account = rpc.get_account(context.merkle_tree).await?.ok_or_else(|| {
                anyhow::anyhow!("Merkle tree account not found")
            })?;

            let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
                account.data.as_mut_slice(),
                &context.merkle_tree.into(),
            )?;

            let (merkle_tree_data, is_ready) = parse_merkle_tree_batch(
                &merkle_tree,
            )?;

            drop(rpc);

            if !is_ready {
                debug!("AddressV2 tree not ready for processing");
                return Ok(0);
            }

            debug!("Processing AddressV2 tree with old method (get_batch_address_update_info)");
            address::process_address_tree(&context, merkle_tree_data).await
        }
        _ => Err(anyhow::anyhow!("Unsupported tree type: {:?}", tree_type)),
    }
}

pub use common::BatchContext;
