use anyhow::anyhow;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_client::{
    indexer::{Indexer, QueueElementsV2Options},
    rpc::Rpc,
};
use tracing::warn;

use crate::processor::v2::BatchContext;

/// Fetches zkp_batch_size from on-chain merkle tree account (called once at startup)
pub async fn fetch_zkp_batch_size<R: Rpc>(context: &BatchContext<R>) -> crate::Result<u64> {
    let rpc = context.rpc_pool.get_connection().await?;
    let mut account = rpc
        .get_account(context.merkle_tree)
        .await?
        .ok_or_else(|| anyhow!("Merkle tree account not found"))?;

    let tree = BatchedMerkleTreeAccount::state_from_bytes(
        account.data.as_mut_slice(),
        &context.merkle_tree.into(),
    )?;

    let batch_index = tree.queue_batches.pending_batch_index;
    let batch = tree
        .queue_batches
        .batches
        .get(batch_index as usize)
        .ok_or_else(|| anyhow!("Batch not found"))?;

    Ok(batch.zkp_batch_size)
}

pub async fn fetch_batches<R: Rpc>(
    context: &BatchContext<R>,
    output_start_index: Option<u64>,
    input_start_index: Option<u64>,
    fetch_len: u64,
    zkp_batch_size: u64,
) -> crate::Result<Option<light_client::indexer::StateQueueDataV2>> {
    let fetch_len_u16: u16 = match fetch_len.try_into() {
        Ok(v) => v,
        Err(_) => {
            warn!(
                "fetch_len {} exceeds u16::MAX, clamping to {}",
                fetch_len,
                u16::MAX
            );
            u16::MAX
        }
    };
    let zkp_batch_size_u16: u16 = match zkp_batch_size.try_into() {
        Ok(v) => v,
        Err(_) => {
            warn!(
                "zkp_batch_size {} exceeds u16::MAX, clamping to {}",
                zkp_batch_size,
                u16::MAX
            );
            u16::MAX
        }
    };

    let mut rpc = context.rpc_pool.get_connection().await?;
    let indexer = rpc.indexer_mut()?;
    let options = QueueElementsV2Options::default()
        .with_output_queue(output_start_index, Some(fetch_len_u16))
        .with_output_queue_batch_size(Some(zkp_batch_size_u16))
        .with_input_queue(input_start_index, Some(fetch_len_u16))
        .with_input_queue_batch_size(Some(zkp_batch_size_u16));

    let res = indexer
        .get_queue_elements_v2(context.merkle_tree.to_bytes(), options, None)
        .await?;

    Ok(res.value.state_queue)
}
