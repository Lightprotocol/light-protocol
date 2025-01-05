use borsh::BorshSerialize;
use forester_utils::{
    indexer::Indexer,
    instructions::{create_append_batch_ix_data, create_nullify_batch_ix_data},
};
use light_batched_merkle_tree::{
    event::{BatchAppendEvent, BatchNullifyEvent},
    queue::BatchedQueueAccount,
};
use light_client::rpc::RpcConnection;
use light_registry::account_compression_cpi::sdk::{
    create_batch_append_instruction, create_batch_nullify_instruction,
};
use solana_sdk::signer::Signer;
use tracing::info;

use super::common::BatchContext;
use crate::batch_processor::error::{BatchProcessError, Result};

pub(crate) async fn process_batch<R: RpcConnection, I: Indexer<R>>(
    context: &BatchContext<R, I>,
) -> Result<usize> {
    info!("Processing state batch append operation");
    let mut rpc = context.rpc_pool.get_connection().await?;

    let (num_inserted_zkps, zkp_batch_size) = {
        let mut output_queue_account = rpc.get_account(context.output_queue).await?.unwrap();
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

    perform_append(context, &mut rpc, num_inserted_zkps).await?;
    info!("Append operation completed");

    perform_nullify(context, &mut rpc).await?;
    info!("Nullify operation completed");

    Ok(zkp_batch_size * 2)
}

async fn perform_append<R: RpcConnection, I: Indexer<R>>(
    context: &BatchContext<R, I>,
    rpc: &mut R,
    num_inserted_zkps: u64,
) -> Result<()> {
    let instruction_data = create_append_batch_ix_data(
        rpc,
        &mut *context.indexer.lock().await,
        context.merkle_tree,
        context.output_queue,
    )
    .await
    .map_err(|e| BatchProcessError::InstructionData(e.to_string()))?;

    let instruction = create_batch_append_instruction(
        context.authority.pubkey(),
        context.derivation,
        context.merkle_tree,
        context.output_queue,
        context.epoch,
        instruction_data
            .try_to_vec()
            .map_err(|e| BatchProcessError::InstructionData(e.to_string()))?,
    );

    rpc.create_and_send_transaction_with_event::<BatchAppendEvent>(
        &[instruction],
        &context.authority.pubkey(),
        &[&context.authority],
        None,
    )
    .await?;

    let mut indexer = context.indexer.lock().await;
    indexer
        .update_test_indexer_after_append(
            rpc,
            context.merkle_tree,
            context.output_queue,
            num_inserted_zkps,
        )
        .await;

    Ok(())
}

async fn perform_nullify<R: RpcConnection, I: Indexer<R>>(
    context: &BatchContext<R, I>,
    rpc: &mut R,
) -> Result<()> {
    let batch_index = get_batch_index(context, rpc).await?;

    let instruction_data =
        create_nullify_batch_ix_data(rpc, &mut *context.indexer.lock().await, context.merkle_tree)
            .await
            .map_err(|e| BatchProcessError::InstructionData(e.to_string()))?;

    let instruction = create_batch_nullify_instruction(
        context.authority.pubkey(),
        context.derivation,
        context.merkle_tree,
        context.epoch,
        instruction_data
            .try_to_vec()
            .map_err(|e| BatchProcessError::InstructionData(e.to_string()))?,
    );

    rpc.create_and_send_transaction_with_event::<BatchNullifyEvent>(
        &[instruction],
        &context.authority.pubkey(),
        &[&context.authority],
        None,
    )
    .await?;

    context
        .indexer
        .lock()
        .await
        .update_test_indexer_after_nullification(rpc, context.merkle_tree, batch_index)
        .await;

    Ok(())
}

async fn get_batch_index<R: RpcConnection, I: Indexer<R>>(
    context: &BatchContext<R, I>,
    rpc: &mut R,
) -> Result<usize> {
    let mut account = rpc.get_account(context.merkle_tree).await?.unwrap();
    let merkle_tree = light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount::state_tree_from_bytes_mut(
        account.data.as_mut_slice(),
    ).map_err(|e| BatchProcessError::MerkleTreeParsing(e.to_string()))?;

    Ok(merkle_tree
        .get_metadata()
        .queue_metadata
        .next_full_batch_index as usize)
}
