use std::sync::Arc;

use borsh::BorshSerialize;
use forester_utils::{
    error::ForesterUtilsError,
    instructions::address_batch_update::create_batch_update_address_tree_instruction_data,
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_merkle_tree_metadata::events::MerkleTreeEvent;
use light_registry::account_compression_cpi::sdk::create_batch_update_address_tree_instruction;
use solana_sdk::signer::Signer;
use tokio::sync::Mutex;
use tracing::{debug, info, instrument, log::error, trace};

use super::{
    common::BatchContext,
    error::{BatchProcessError, Result},
};
use crate::indexer_type::{finalize_batch_address_tree_update, IndexerType};

#[instrument(level = "debug", skip(context), fields(tree = %context.merkle_tree))]
pub(crate) async fn process_batch<R: Rpc, I: Indexer + IndexerType<R>>(
    context: &BatchContext<R, I>,
) -> Result<usize> {
    trace!("Processing address batch operation");

    let total_items_processed = Arc::new(Mutex::new(0usize));
    let tx_counter = Arc::new(Mutex::new(0usize));

    let context_clone = context;
    let total_items_processed_clone = total_items_processed.clone();
    let tx_counter_clone = tx_counter.clone();

    let tx_callback = move |instruction_batch: Vec<
        light_batched_merkle_tree::merkle_tree::InstructionDataAddressAppendInputs,
    >,
                            zkp_batch_size: u16| {
        let context = context_clone;
        let total_items_processed = total_items_processed_clone.clone();
        let tx_counter = tx_counter_clone.clone();

        async move {
            let mut counter = tx_counter.lock().await;
            *counter += 1;
            let current_tx = *counter;
            drop(counter);

            debug!(
                "Sending address update transaction {} for tree: {}",
                current_tx, context.merkle_tree
            );

            // This is also a pooled connection, so it must be dereferenced.
            let mut rpc = context.rpc_pool.get_connection().await.map_err(|e| {
                error!("Failed to get RPC connection: {:?}", e);
                ForesterUtilsError::Rpc(e.to_string())
            })?;

            let mut instructions = Vec::with_capacity(instruction_batch.len());
            for instruction_data in instruction_batch.iter() {
                let serialized_data = instruction_data.try_to_vec().map_err(|e| {
                    error!("Failed to serialize instruction data: {:?}", e);
                    ForesterUtilsError::Prover(format!(
                        "Failed to serialize instruction data: {}",
                        e
                    ))
                })?;

                instructions.push(create_batch_update_address_tree_instruction(
                    context.authority.pubkey(),
                    context.derivation,
                    context.merkle_tree,
                    context.epoch,
                    serialized_data,
                ));
            }

            // The create_and_send... method is on the Rpc trait, so deref is needed.
            let tx = rpc
                .create_and_send_transaction_with_event::<MerkleTreeEvent>(
                    &instructions,
                    &context.authority.pubkey(),
                    &[&context.authority],
                )
                .await
                .map_err(|e| {
                    error!(
                        "Failed to send address update transaction {} for tree {}: {:?}",
                        current_tx, context.merkle_tree, e
                    );
                    ForesterUtilsError::Rpc(e.to_string())
                })?;

            info!(
                "Address update transaction {} sent successfully: {:?}",
                current_tx, tx
            );

            let mut total = total_items_processed.lock().await;
            *total += instruction_batch.len() * zkp_batch_size as usize;
            debug!(
                "Callback completed: processed {} items. Total now: {}",
                instruction_batch.len() * zkp_batch_size as usize,
                *total
            );

            Ok(())
        }
    };

    info!(
        "Starting address processing for tree {} with ixs_per_tx={}",
        context.merkle_tree, context.ixs_per_tx
    );

    let mut rpc = context.rpc_pool.get_connection().await?;
    let zkp_batches_created = create_batch_update_address_tree_instruction_data(
        // FIX #1: Dereference the pooled connection here
        &mut *rpc,
        &mut *context.indexer.lock().await,
        &context.merkle_tree,
        context.prover_url.clone(),
        context.prover_polling_interval,
        context.prover_max_wait_time,
        context.ixs_per_tx,
        tx_callback,
    )
    .await
    .map_err(|e| {
        error!(
            "Failed to create batch update address tree instruction data: {}",
            e
        );
        BatchProcessError::InstructionData(e.to_string())
    })?;

    if zkp_batches_created == 0 {
        trace!("No ZKP batches to process for address tree");
        let mut cache = context.ops_cache.lock().await;
        cache.cleanup();
        return Ok(0);
    }

    debug!("Starting finalization after all transactions have been sent...");
    finalize_batch_address_tree_update(
        // FIX #2: Dereference the pooled connection here as well
        &mut *rpc,
        context.indexer.clone(),
        context.merkle_tree,
    )
    .await
    .map_err(|e| {
        error!("Failed to finalize batch address tree update: {:?}", e);
        BatchProcessError::Indexer(e.to_string())
    })?;
    debug!("Batch finalization successful.");

    let final_total = *total_items_processed.lock().await;
    let final_tx_count = *tx_counter.lock().await;

    info!(
        "Address batch processing completed successfully. Processed {} items across {} transactions",
        final_total, final_tx_count
    );

    Ok(final_total)
}
