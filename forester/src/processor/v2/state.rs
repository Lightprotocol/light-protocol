use borsh::BorshSerialize;
use forester_utils::instructions::state_batch_append::get_append_instruction_stream;
use forester_utils::instructions::state_batch_nullify::get_nullify_instruction_stream;
use futures::{pin_mut, StreamExt};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_registry::account_compression_cpi::sdk::{
    create_batch_append_instruction, create_batch_nullify_instruction,
};
use solana_sdk::signer::Signer;
use tracing::instrument;

use super::common::BatchContext;
use crate::indexer_type::{
    update_test_indexer_after_append, update_test_indexer_after_nullification, IndexerType,
};
use crate::Result;

#[instrument(level = "debug", skip(context))]
pub(crate) async fn perform_append<R: Rpc, I: Indexer + IndexerType<R>>(
    context: &BatchContext<R, I>,
) -> Result<()> {
    let (instruction_stream, _) = get_append_instruction_stream(
        context.rpc_pool.clone(),
        context.indexer.clone(),
        context.merkle_tree,
        context.output_queue,
        context.prover_url.clone(),
        context.prover_polling_interval,
        context.prover_max_wait_time,
    )
    .await?;

    pin_mut!(instruction_stream);
    let mut instruction_buffer = Vec::new();

    let mut rpc = context.rpc_pool.get_connection().await?;

    while let Some(result) = instruction_stream.next().await {
        instruction_buffer.push(result?);
        if instruction_buffer.len() >= context.ixs_per_tx {
            let instructions = instruction_buffer
                .iter()
                .map(|data| {
                    create_batch_append_instruction(
                        context.authority.pubkey(),
                        context.derivation,
                        context.merkle_tree,
                        context.output_queue,
                        context.epoch,
                        data.try_to_vec().unwrap(),
                    )
                })
                .collect::<Vec<_>>();
            rpc.create_and_send_transaction(
                &instructions,
                &context.authority.pubkey(),
                &[&context.authority],
            )
            .await?;
            instruction_buffer.clear();
        }
    }

    if !instruction_buffer.is_empty() {
        let instructions = instruction_buffer
            .iter()
            .map(|data| {
                create_batch_append_instruction(
                    context.authority.pubkey(),
                    context.derivation,
                    context.merkle_tree,
                    context.output_queue,
                    context.epoch,
                    data.try_to_vec().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        rpc.create_and_send_transaction(
            &instructions,
            &context.authority.pubkey(),
            &[&context.authority],
        )
        .await?;
    }

    update_test_indexer_after_append(
        &mut *rpc,
        context.indexer.clone(),
        context.merkle_tree,
        context.output_queue,
    )
    .await?;

    Ok(())
}

#[instrument(level = "debug", skip(context))]
pub(crate) async fn perform_nullify<R: Rpc, I: Indexer + IndexerType<R>>(
    context: &BatchContext<R, I>,
) -> Result<()> {
    let batch_index = get_batch_index(context).await?;
    let (instruction_stream, _) = get_nullify_instruction_stream(
        context.rpc_pool.clone(),
        context.indexer.clone(),
        context.merkle_tree,
        context.prover_url.clone(),
        context.prover_polling_interval,
        context.prover_max_wait_time,
    )
    .await?;

    pin_mut!(instruction_stream);
    let mut instruction_buffer = Vec::new();

    let mut rpc = context.rpc_pool.get_connection().await?;

    while let Some(result) = instruction_stream.next().await {
        instruction_buffer.push(result?);
        if instruction_buffer.len() >= context.ixs_per_tx {
            let instructions = instruction_buffer
                .iter()
                .map(|data| {
                    create_batch_nullify_instruction(
                        context.authority.pubkey(),
                        context.derivation,
                        context.merkle_tree,
                        context.epoch,
                        data.try_to_vec().unwrap(),
                    )
                })
                .collect::<Vec<_>>();
            rpc.create_and_send_transaction(
                &instructions,
                &context.authority.pubkey(),
                &[&context.authority],
            )
            .await?;
            instruction_buffer.clear();
        }
    }

    if !instruction_buffer.is_empty() {
        let instructions = instruction_buffer
            .iter()
            .map(|data| {
                create_batch_nullify_instruction(
                    context.authority.pubkey(),
                    context.derivation,
                    context.merkle_tree,
                    context.epoch,
                    data.try_to_vec().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        rpc.create_and_send_transaction(
            &instructions,
            &context.authority.pubkey(),
            &[&context.authority],
        )
        .await?;
    }

    update_test_indexer_after_nullification(
        &mut *rpc,
        context.indexer.clone(),
        context.merkle_tree,
        batch_index,
    )
    .await?;

    Ok(())
}

async fn get_batch_index<R: Rpc, I: Indexer>(context: &BatchContext<R, I>) -> Result<usize> {
    let rpc = context.rpc_pool.get_connection().await?;
    let mut account = rpc
        .get_account(context.merkle_tree)
        .await?
        .ok_or_else(|| anyhow::anyhow!("State tree account not found: {}", context.merkle_tree))?;

    let merkle_tree =
        light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount::state_from_bytes(
            account.data.as_mut_slice(),
            &context.merkle_tree.into(),
        )?;

    Ok(merkle_tree.queue_batches.pending_batch_index as usize)
}
