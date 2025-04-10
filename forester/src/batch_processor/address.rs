use borsh::BorshSerialize;
use forester_utils::instructions::address_batch_update::create_batch_update_address_tree_instruction_data;
use light_client::{indexer::Indexer, rpc::RpcConnection};
use light_merkle_tree_metadata::events::MerkleTreeEvent;
use light_registry::account_compression_cpi::sdk::create_batch_update_address_tree_instruction;
use solana_sdk::signer::Signer;
use tracing::{debug, info, instrument, log::error};

use super::common::BatchContext;
use crate::{
    batch_processor::error::{BatchProcessError, Result},
    indexer_type::{finalize_batch_address_tree_update, IndexerType},
};

#[instrument(level = "debug", skip(context), fields(tree = %context.merkle_tree))]
pub(crate) async fn process_batch<R: RpcConnection, I: Indexer<R> + IndexerType<R>>(
    context: &BatchContext<R, I>,
) -> Result<usize> {
    info!("Processing address batch operation");
    let mut rpc = context.rpc_pool.get_connection().await?;

    let (instruction_data_vec, zkp_batch_size) = create_batch_update_address_tree_instruction_data(
        &mut *rpc,
        &mut *context.indexer.lock().await,
        &context.merkle_tree,
    )
    .await
    .map_err(|e| {
        error!(
            "Failed to create batch update address tree instruction data: {}",
            e
        );
        BatchProcessError::InstructionData(e.to_string())
    })?;

    if instruction_data_vec.is_empty() {
        debug!("No ZKP batches to process for address tree");
        return Ok(0);
    }

    info!(
        "Processing {} ZKP batch updates for address tree",
        instruction_data_vec.len()
    );

    let mut batches_processed = 0;

    for (chunk_idx, instruction_chunk) in
        instruction_data_vec.chunks(context.ixs_per_tx).enumerate()
    {
        debug!(
            "Sending address update transaction chunk {}/{} for tree: {}",
            chunk_idx + 1,
            instruction_data_vec.len().div_ceil(context.ixs_per_tx),
            context.merkle_tree
        );

        let mut instructions = Vec::with_capacity(context.ixs_per_tx);
        for instruction_data in instruction_chunk {
            debug!(
                "Instruction data size: {} bytes",
                instruction_data.try_to_vec().map(|v| v.len()).unwrap_or(0)
            );

            instructions.push(create_batch_update_address_tree_instruction(
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
            ));

            batches_processed += 1;
        }

        let tx = match rpc
            .create_and_send_transaction_with_event::<MerkleTreeEvent>(
                &instructions,
                &context.authority.pubkey(),
                &[&context.authority],
                None,
            )
            .await
        {
            Ok(tx) => {
                info!(
                    "Address update transaction chunk {}/{} sent successfully: {:?}",
                    chunk_idx + 1,
                    instruction_data_vec.len().div_ceil(context.ixs_per_tx),
                    tx
                );
                tx
            }
            Err(e) => {
                error!(
                    "Failed to send address update transaction chunk {}/{} for tree {}: {:?}",
                    chunk_idx + 1,
                    instruction_data_vec.len().div_ceil(context.ixs_per_tx),
                    context.merkle_tree,
                    e
                );
                return Err(e.into());
            }
        };

        debug!("Address batch transaction: {:?}", tx);

        finalize_batch_address_tree_update(&mut *rpc, context.indexer.clone(), context.merkle_tree)
            .await
            .map_err(|e| {
                error!("Failed to finalize batch address tree update: {:?}", e);
                BatchProcessError::Indexer(e.to_string())
            })?;
    }

    info!(
        "Address batch processing completed successfully. Processed {} batches",
        batches_processed
    );

    Ok(batches_processed * zkp_batch_size as usize)
}
