use std::future::Future;

use borsh::BorshSerialize;
use forester_utils::utils::wait_for_indexer;
use futures::{pin_mut, stream::StreamExt, Stream};
use light_client::rpc::Rpc;
use light_registry::protocol_config::state::EpochState;
use solana_sdk::{instruction::Instruction, signer::Signer};
use tracing::{info, trace};

use super::context::BatchContext;
use crate::{errors::ForesterError, Result};

/// Processes a stream of batched instruction data into transactions.
pub(crate) async fn process_stream<R, S, D, FutC>(
    context: &BatchContext<R>,
    stream_creator_future: FutC,
    instruction_builder: impl Fn(&D) -> Instruction,
) -> Result<usize>
where
    R: Rpc,
    S: Stream<Item = Result<Vec<D>>> + Send,
    D: BorshSerialize,
    FutC: Future<Output = Result<(S, u16)>> + Send,
{
    trace!("Executing batched stream processor (hybrid)");

    let (batch_stream, zkp_batch_size) = stream_creator_future.await?;

    if zkp_batch_size == 0 {
        trace!("ZKP batch size is 0, no work to do.");
        return Ok(0);
    }

    pin_mut!(batch_stream);
    let mut total_instructions_processed = 0;

    while let Some(batch_result) = batch_stream.next().await {
        let instruction_batch = batch_result?;

        if instruction_batch.is_empty() {
            continue;
        }

        let current_slot = context.slot_tracker.estimated_current_slot();
        let phase_end_slot = context.epoch_phases.active.end;
        let slots_remaining = phase_end_slot.saturating_sub(current_slot);

        const MIN_SLOTS_FOR_TRANSACTION: u64 = 30;
        if slots_remaining < MIN_SLOTS_FOR_TRANSACTION {
            info!(
                "Only {} slots remaining in active phase (need at least {}), stopping batch processing",
                slots_remaining, MIN_SLOTS_FOR_TRANSACTION
            );
            if !instruction_batch.is_empty() {
                let instructions: Vec<Instruction> =
                    instruction_batch.iter().map(&instruction_builder).collect();
                let _ = send_transaction_batch(context, instructions).await;
            }
            break;
        }

        let instructions: Vec<Instruction> =
            instruction_batch.iter().map(&instruction_builder).collect();

        match send_transaction_batch(context, instructions).await {
            Ok(_) => {
                total_instructions_processed += instruction_batch.len();
                {
                    let rpc = context.rpc_pool.get_connection().await?;
                    wait_for_indexer(&*rpc)
                        .await
                        .map_err(|e| anyhow::anyhow!("Error: {:?}", e))?;
                }
            }
            Err(e) => {
                if let Some(ForesterError::NotInActivePhase) = e.downcast_ref::<ForesterError>() {
                    info!("Active phase ended while processing batches, stopping gracefully");
                    break;
                } else {
                    return Err(e);
                }
            }
        }
    }

    if total_instructions_processed == 0 {
        trace!("No instructions were processed from the stream.");
        return Ok(0);
    }

    let total_items_processed = total_instructions_processed * zkp_batch_size as usize;
    Ok(total_items_processed)
}

pub(crate) async fn send_transaction_batch<R: Rpc>(
    context: &BatchContext<R>,
    instructions: Vec<Instruction>,
) -> Result<String> {
    // Check if we're still in the active phase before sending the transaction
    let current_slot = context.slot_tracker.estimated_current_slot();
    let current_phase_state = context.epoch_phases.get_current_epoch_state(current_slot);

    if current_phase_state != EpochState::Active {
        trace!(
            "Skipping transaction send: not in active phase (current phase: {:?}, slot: {})",
            current_phase_state,
            current_slot
        );
        return Err(ForesterError::NotInActivePhase.into());
    }

    info!(
        "Sending transaction with {} instructions...",
        instructions.len()
    );

    let mut rpc = context.rpc_pool.get_connection().await?;
    let signature = rpc
        .create_and_send_transaction(
            &instructions,
            &context.authority.pubkey(),
            &[&context.authority],
        )
        .await?;

    Ok(signature.to_string())
}
