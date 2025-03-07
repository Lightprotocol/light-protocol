use borsh::BorshSerialize;
use forester_utils::instructions::address_batch_update::create_batch_update_address_tree_instruction_data;
use light_client::{indexer::Indexer, rpc::RpcConnection};
use light_merkle_tree_metadata::events::MerkleTreeEvent;
use light_registry::account_compression_cpi::sdk::create_batch_update_address_tree_instruction;
use solana_sdk::signer::Signer;
use tracing::{debug, instrument};

use super::common::BatchContext;
use crate::{
    batch_processor::error::{BatchProcessError, Result},
    indexer_type::{finalize_batch_address_tree_update, IndexerType},
};

#[instrument(level = "debug", skip(context), fields(tree = %context.merkle_tree))]
pub(crate) async fn process_batch<R: RpcConnection, I: Indexer<R> + IndexerType<R>>(
    context: &BatchContext<R, I>,
) -> Result<usize> {
    debug!("Processing address batch operation");
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

    let tx = rpc
        .create_and_send_transaction_with_event::<MerkleTreeEvent>(
            &[instruction],
            &context.authority.pubkey(),
            &[&context.authority],
            None,
        )
        .await?;
    debug!("tx address BatchNullifyEvent: {:?}", tx);

    finalize_batch_address_tree_update(&mut *rpc, context.indexer.clone(), context.merkle_tree)
        .await
        .expect("Failed to finalize batch address tree update");

    debug!(
        "Address batch processing completed successfully. Batch size: {}",
        batch_size
    );
    Ok(batch_size)
}
