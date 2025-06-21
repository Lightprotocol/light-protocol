use crate::{
    errors::ForesterError,
    indexer_type::{finalize_batch_address_tree_update, IndexerType},
    processor::v2::common::BatchContext,
    Result,
};
use borsh::BorshSerialize;
use forester_utils::instructions::address_batch_update::get_address_update_stream;
use futures::{pin_mut, stream::StreamExt};
use light_batched_merkle_tree::merkle_tree::InstructionDataAddressAppendInputs;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_merkle_tree_metadata::events::MerkleTreeEvent;
use light_registry::account_compression_cpi::sdk::create_batch_update_address_tree_instruction;
use solana_sdk::signer::Signer;
use tracing::{debug, info, instrument, trace};

async fn send_transaction_batch<R: Rpc, I: Indexer>(
    instructions_to_send: &[InstructionDataAddressAppendInputs],
    context: &BatchContext<R, I>,
    tx_num: usize,
) -> Result<()> {
    debug!(
        "Sending address update transaction {} with {} instructions",
        tx_num,
        instructions_to_send.len()
    );

    let mut rpc = context.rpc_pool.get_connection().await?;

    let mut instructions = Vec::new();
    for instruction_data in instructions_to_send {
        let serialized_data = instruction_data.try_to_vec()?;
        instructions.push(create_batch_update_address_tree_instruction(
            context.authority.pubkey(),
            context.derivation,
            context.merkle_tree,
            context.epoch,
            serialized_data,
        ));
    }

    rpc.create_and_send_transaction_with_event::<MerkleTreeEvent>(
        &instructions,
        &context.authority.pubkey(),
        &[&context.authority],
    )
    .await?;

    info!("Address update transaction {} sent successfully", tx_num);
    Ok(())
}

#[instrument(level = "debug", skip(context), fields(tree = %context.merkle_tree))]
pub(crate) async fn process_batch<R: Rpc, I: Indexer + IndexerType<R>>(
    context: &BatchContext<R, I>,
) -> Result<usize> {
    trace!("Processing address batch operation");

    let mut rpc_conn = context.rpc_pool.get_connection().await?;

    let mut binding = context.indexer.lock().await;
    let (instruction_stream, zkp_batch_size) = get_address_update_stream(
        &mut *rpc_conn,
        &mut *binding,
        &context.merkle_tree,
        context.prover_url.clone(),
        context.prover_polling_interval,
        context.prover_max_wait_time,
    )
    .await?;

    if zkp_batch_size == 0 {
        trace!("ZKP batch size is 0, no work to do.");
        return Ok(0);
    }

    pin_mut!(instruction_stream);

    let mut instruction_buffer = Vec::new();
    let mut total_instructions_processed = 0;
    let mut tx_counter = 0;

    while let Some(result) = instruction_stream.next().await {
        let instruction_data = result?;
        instruction_buffer.push(instruction_data);
        total_instructions_processed += 1;

        if instruction_buffer.len() >= context.ixs_per_tx {
            tx_counter += 1;
            send_transaction_batch(&instruction_buffer, context, tx_counter).await?;
            instruction_buffer.clear();
        }
    }

    if !instruction_buffer.is_empty() {
        tx_counter += 1;
        send_transaction_batch(&instruction_buffer, context, tx_counter).await?;
    }

    if total_instructions_processed == 0 {
        trace!("No ZKP batches to process for address tree");
        return Ok(0);
    }

    debug!("Finalizing batch update after all transactions are sent...");
    finalize_batch_address_tree_update(
        &mut *rpc_conn,
        context.indexer.clone(),
        context.merkle_tree,
    )
    .await?;
    debug!("Batch finalization successful.");

    let total_items_processed = total_instructions_processed * zkp_batch_size as usize;

    info!(
        "Address batch processing completed. Processed {} items ({} instructions * batch size {}) across {} transactions.",
        total_items_processed, total_instructions_processed, zkp_batch_size, tx_counter
    );

    Ok(total_items_processed)
}
