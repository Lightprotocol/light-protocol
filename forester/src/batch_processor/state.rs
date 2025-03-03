use borsh::BorshSerialize;
use forester_utils::instructions::{create_append_batch_ix_data, create_nullify_batch_ix_data};
use light_client::{indexer::Indexer, rpc::RpcConnection};
use light_registry::account_compression_cpi::sdk::{
    create_batch_append_instruction, create_batch_nullify_instruction,
};
use solana_sdk::signer::Signer;
use tracing::debug;

use super::common::BatchContext;
use crate::{
    batch_processor::error::{BatchProcessError, Result},
    indexer_type::{
        update_test_indexer_after_append, update_test_indexer_after_nullification, IndexerType,
    },
};

pub(crate) async fn perform_append<R: RpcConnection, I: Indexer<R> + IndexerType<R>>(
    context: &BatchContext<R, I>,
    rpc: &mut R,
) -> Result<()> {
    debug!("perform_append");
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

    rpc.create_and_send_transaction(
        &[instruction],
        &context.authority.pubkey(),
        &[&context.authority],
    )
    .await?;

    update_test_indexer_after_append(
        rpc,
        context.indexer.clone(),
        context.merkle_tree,
        context.output_queue,
    )
    .await
    .expect("Failed to update test indexer after append");

    Ok(())
}

pub(crate) async fn perform_nullify<R: RpcConnection, I: Indexer<R> + IndexerType<R>>(
    context: &BatchContext<R, I>,
    rpc: &mut R,
) -> Result<()> {
    debug!("perform_nullify");
    let batch_index = get_batch_index(context, rpc).await?;
    debug!("batch_index: {:?}", batch_index);
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

    rpc.create_and_send_transaction(
        &[instruction],
        &context.authority.pubkey(),
        &[&context.authority],
    )
    .await?;

    update_test_indexer_after_nullification(
        rpc,
        context.indexer.clone(),
        context.merkle_tree,
        batch_index,
    )
    .await
    .expect("Failed to update test indexer after nullification");

    Ok(())
}

async fn get_batch_index<R: RpcConnection, I: Indexer<R>>(
    context: &BatchContext<R, I>,
    rpc: &mut R,
) -> Result<usize> {
    let mut account = rpc.get_account(context.merkle_tree).await?.unwrap();
    let merkle_tree =
        light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount::state_from_bytes(
            account.data.as_mut_slice(),
            &context.merkle_tree.into(),
        )
        .map_err(|e| BatchProcessError::MerkleTreeParsing(e.to_string()))?;

    Ok(merkle_tree.queue_batches.pending_batch_index as usize)
}
