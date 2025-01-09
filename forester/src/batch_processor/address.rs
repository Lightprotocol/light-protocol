use borsh::BorshSerialize;
use forester_utils::{
    indexer::Indexer, instructions::create_batch_update_address_tree_instruction_data,
};
use light_batched_merkle_tree::event::BatchNullifyEvent;
use light_client::rpc::RpcConnection;
use light_registry::account_compression_cpi::sdk::create_batch_update_address_tree_instruction;
use solana_sdk::signer::Signer;
use tracing::{info, instrument};

use super::common::BatchContext;
use crate::batch_processor::error::{BatchProcessError, Result};

#[instrument(level = "debug", skip(context), fields(tree = %context.merkle_tree))]
pub(crate) async fn process_batch<R: RpcConnection, I: Indexer<R>>(
    context: &BatchContext<R, I>,
) -> Result<usize> {
    info!("Processing address batch operation");
    let mut rpc = context.rpc_pool.get_connection().await?;

    // Create instruction data and get batch size
    let (instruction_data, batch_size) = create_batch_update_address_tree_instruction_data(
        &mut *rpc,
        &mut *context.indexer.lock().await,
        context.merkle_tree,
    )
    .await
    .map_err(|e| BatchProcessError::InstructionData(e.to_string()))?;

    // Create the instruction
    let instruction = create_batch_update_address_tree_instruction(
        context.authority.pubkey(),
        context.derivation,
        context.merkle_tree,
        context.epoch,
        instruction_data.try_to_vec().map_err(|e| {
            BatchProcessError::InstructionData(format!(
                "Failed to serialize instruction data: {}",
                e
            ))
        })?,
    );

    rpc.create_and_send_transaction_with_event::<BatchNullifyEvent>(
        &[instruction],
        &context.authority.pubkey(),
        &[&context.authority],
        None,
    )
    .await?;

    let mut indexer = context.indexer.lock().await;
    indexer
        .finalize_batched_address_tree_update(&mut *rpc, context.merkle_tree)
        .await;

    info!(
        "Address batch processing completed successfully. Batch size: {}",
        batch_size
    );
    Ok(batch_size)
}
