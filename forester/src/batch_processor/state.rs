use borsh::BorshSerialize;
use forester_utils::instructions::{
    state_batch_append::create_append_batch_ix_data,
    state_batch_nullify::create_nullify_batch_ix_data,
};
use light_client::{indexer::Indexer, rpc::RpcConnection};
use light_registry::account_compression_cpi::sdk::{
    create_batch_append_instruction, create_batch_nullify_instruction,
};
use solana_sdk::signer::Signer;
use tracing::{debug, info, instrument, log::error};

use super::common::BatchContext;
use crate::{
    batch_processor::error::{BatchProcessError, Result},
    indexer_type::{
        update_test_indexer_after_append, update_test_indexer_after_nullification, IndexerType,
    },
};

#[instrument(
    level = "debug",
    fields(
        forester = %context.derivation,
        epoch = %context.derivation,
        merkle_tree = %context.merkle_tree,
        output_queue = %context.output_queue,
    ), skip(context, rpc))
]
pub(crate) async fn perform_append<R: RpcConnection, I: Indexer<R> + IndexerType<R>>(
    context: &BatchContext<R, I>,
    rpc: &mut R,
) -> Result<()> {
    let instruction_data_vec = create_append_batch_ix_data(
        rpc,
        &mut *context.indexer.lock().await,
        context.merkle_tree,
        context.output_queue,
    )
    .await
    .map_err(|e| {
        error!("Failed to create append batch instruction data: {}", e);
        BatchProcessError::InstructionData(e.to_string())
    })?;

    if instruction_data_vec.is_empty() {
        debug!("No zkp batches to append");
        return Ok(());
    }

    info!(
        "Processing {} ZKP batch appends",
        instruction_data_vec.len()
    );

    let chunk_size = 7;
    for (chunk_idx, instruction_chunk) in instruction_data_vec.chunks(chunk_size).enumerate() {
        debug!(
            "Sending append transaction chunk {}/{} for tree: {}",
            chunk_idx + 1,
            instruction_data_vec.len().div_ceil(chunk_size),
            context.merkle_tree
        );

        let mut instructions = Vec::with_capacity(chunk_size);
        for instruction_data in instruction_chunk {
            debug!(
                "Instruction data size: {} bytes",
                instruction_data.try_to_vec().map(|v| v.len()).unwrap_or(0)
            );

            instructions.push(create_batch_append_instruction(
                context.authority.pubkey(),
                context.derivation,
                context.merkle_tree,
                context.output_queue,
                context.epoch,
                instruction_data
                    .try_to_vec()
                    .map_err(|e| BatchProcessError::InstructionData(e.to_string()))?,
            ));
        }

        match rpc
            .create_and_send_transaction(
                &instructions,
                &context.authority.pubkey(),
                &[&context.authority],
            )
            .await
        {
            Ok(tx) => {
                info!(
                    "Append transaction chunk {}/{} sent successfully: {}",
                    chunk_idx + 1,
                    instruction_data_vec.len().div_ceil(chunk_size),
                    tx
                );
            }
            Err(e) => {
                error!(
                    "Failed to send append transaction chunk {}/{} for tree {}: {:?}",
                    chunk_idx + 1,
                    instruction_data_vec.len().div_ceil(chunk_size),
                    context.merkle_tree,
                    e
                );
                return Err(e.into());
            }
        }

        update_test_indexer_after_append(
            rpc,
            context.indexer.clone(),
            context.merkle_tree,
            context.output_queue,
        )
        .await
        .map_err(|e| {
            error!("Failed to update test indexer after append: {:?}", e);
            BatchProcessError::Indexer(e.to_string())
        })?;
    }

    Ok(())
}

/// Perform a state nullify operation for a Merkle tree
#[instrument(
    level = "debug",
    fields(
        forester = %context.derivation,
        epoch = %context.epoch,
        merkle_tree = %context.merkle_tree,
    ),
    skip(context, rpc)
)]
pub(crate) async fn perform_nullify<R: RpcConnection, I: Indexer<R> + IndexerType<R>>(
    context: &BatchContext<R, I>,
    rpc: &mut R,
) -> Result<()> {
    let batch_index = get_batch_index(context, rpc).await?;
    let instruction_data_vec =
        create_nullify_batch_ix_data(rpc, &mut *context.indexer.lock().await, context.merkle_tree)
            .await
            .map_err(|e| {
                error!("Failed to create nullify batch instruction data: {}", e);
                BatchProcessError::InstructionData(e.to_string())
            })?;

    if instruction_data_vec.is_empty() {
        debug!("No zkp batches to nullify");
        return Ok(());
    }

    info!(
        "Processing {} ZKP batch nullifications",
        instruction_data_vec.len()
    );

    let chunk_size = 7;
    for (chunk_idx, instruction_chunk) in instruction_data_vec.chunks(chunk_size).enumerate() {
        debug!(
            "Processing nullify transaction chunk {}/{}",
            chunk_idx + 1,
            instruction_data_vec.len().div_ceil(chunk_size)
        );

        let mut instructions = Vec::with_capacity(chunk_size);
        for instruction_data in instruction_chunk {
            instructions.push(create_batch_nullify_instruction(
                context.authority.pubkey(),
                context.derivation,
                context.merkle_tree,
                context.epoch,
                instruction_data
                    .try_to_vec()
                    .map_err(|e| BatchProcessError::InstructionData(e.to_string()))?,
            ));
        }

        match rpc
            .create_and_send_transaction(
                &instructions,
                &context.authority.pubkey(),
                &[&context.authority],
            )
            .await
        {
            Ok(tx) => {
                info!(
                    "Nullify transaction chunk {}/{} sent successfully: {}",
                    chunk_idx + 1,
                    instruction_data_vec.len().div_ceil(chunk_size),
                    tx
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
            Err(e) => {
                error!(
                    "Failed to send nullify transaction chunk {}/{} for tree {}: {:?}",
                    chunk_idx + 1,
                    instruction_data_vec.len().div_ceil(chunk_size),
                    context.merkle_tree,
                    e
                );
                return Err(e.into());
            }
        }

        update_test_indexer_after_nullification(
            rpc,
            context.indexer.clone(),
            context.merkle_tree,
            batch_index,
        )
        .await
        .map_err(|e| {
            error!("Failed to update test indexer after nullification: {:?}", e);
            BatchProcessError::Indexer(e.to_string())
        })?;
    }

    Ok(())
}

/// Get the current batch index from the Merkle tree account
async fn get_batch_index<R: RpcConnection, I: Indexer<R>>(
    context: &BatchContext<R, I>,
    rpc: &mut R,
) -> Result<usize> {
    let mut account = rpc.get_account(context.merkle_tree).await?.ok_or_else(|| {
        BatchProcessError::Rpc(format!("Account not found: {}", context.merkle_tree))
    })?;

    let merkle_tree =
        light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount::state_from_bytes(
            account.data.as_mut_slice(),
            &context.merkle_tree.into(),
        )
        .map_err(|e| BatchProcessError::MerkleTreeParsing(e.to_string()))?;

    Ok(merkle_tree.queue_batches.pending_batch_index as usize)
}
