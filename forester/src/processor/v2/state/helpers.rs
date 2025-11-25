use anyhow::anyhow;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_client::indexer::QueueElementsV2Options;
use light_client::rpc::Rpc;
use crate::processor::v2::BatchContext;
use light_client::indexer::Indexer;

/// Fetches zkp_batch_size from on-chain merkle tree account (called once at startup)
pub async fn fetch_zkp_batch_size<R: Rpc>(context: &BatchContext<R>) -> crate::Result<u16> {
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

    Ok(batch.zkp_batch_size as u16)
}

pub async fn fetch_batches<R: Rpc>(
    context: &BatchContext<R>,
    output_start_index: Option<u64>,
    input_start_index: Option<u64>,
    fetch_len: u16,
    zkp_batch_size: u16,
) -> crate::Result<(
    Option<light_client::indexer::OutputQueueDataV2>,
    Option<light_client::indexer::InputQueueDataV2>,
)> {
    let mut rpc = context.rpc_pool.get_connection().await?;
    let indexer = rpc.indexer_mut()?;
    let options = QueueElementsV2Options::default()
        .with_output_queue(output_start_index, Some(fetch_len))
        .with_output_queue_batch_size(Some(zkp_batch_size))
        .with_input_queue(input_start_index, Some(fetch_len))
        .with_input_queue_batch_size(Some(zkp_batch_size));

    let res = indexer
        .get_queue_elements_v2(context.merkle_tree.to_bytes(), options, None)
        .await?;

    Ok((res.value.output_queue, res.value.input_queue))
}
