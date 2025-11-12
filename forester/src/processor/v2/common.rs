use std::{future::Future, sync::Arc, time::Duration};

use borsh::BorshSerialize;
use forester_utils::{
    forester_epoch::EpochPhases, rpc_pool::SolanaRpcPool, utils::wait_for_indexer,
};
pub use forester_utils::ParsedMerkleTreeData;
use futures::{pin_mut, stream::StreamExt, Stream};
use light_client::rpc::Rpc;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Keypair};
use solana_sdk::signature::Signer;
use tokio::sync::Mutex;
use tracing::{debug, error, info, trace};
use light_registry::protocol_config::state::EpochState;
use crate::{
    errors::ForesterError, processor::tx_cache::ProcessedHashCache, slot_tracker::SlotTracker,
    Result,
};

#[derive(Debug)]
pub struct BatchContext<R: Rpc> {
    pub rpc_pool: Arc<SolanaRpcPool<R>>,
    pub authority: Keypair,
    pub derivation: Pubkey,
    pub epoch: u64,
    pub merkle_tree: Pubkey,
    pub output_queue: Pubkey,
    pub prover_append_url: String,
    pub prover_update_url: String,
    pub prover_address_append_url: String,
    pub prover_api_key: Option<String>,
    pub prover_polling_interval: Duration,
    pub prover_max_wait_time: Duration,
    pub ops_cache: Arc<Mutex<ProcessedHashCache>>,
    pub epoch_phases: EpochPhases,
    pub slot_tracker: Arc<SlotTracker>,
    /// input queue size from gRPC
    pub input_queue_hint: Option<u64>,
    /// output queue size from gRPC
    pub output_queue_hint: Option<u64>,
}

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

        match send_transaction_batch(context, instructions.clone()).await {
            Ok(sig) => {
                total_instructions_processed += instruction_batch.len();
                debug!(
                    "Successfully processed batch with {} instructions, signature: {}",
                    instruction_batch.len(),
                    sig
                );

                {
                    let rpc = context.rpc_pool.get_connection().await?;
                    wait_for_indexer(&*rpc)
                        .await
                        .map_err(|e| anyhow::anyhow!("Error waiting for indexer: {:?}", e))?;
                }
            }
            Err(e) => {
                if let Some(ForesterError::NotInActivePhase) = e.downcast_ref::<ForesterError>() {
                    info!("Active phase ended while processing batches, stopping gracefully");
                    break;
                } else {
                    error!(
                        "Failed to process batch with {} instructions for tree {}: {:?}",
                        instructions.len(),
                        context.merkle_tree,
                        e
                    );
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
        "Sending transaction with {} instructions for tree: {}...",
        instructions.len(),
        context.merkle_tree
    );
    let mut rpc = context.rpc_pool.get_connection().await?;
    let signature = rpc
        .create_and_send_transaction(
            &instructions,
            &context.authority.pubkey(),
            &[&context.authority],
        )
        .await?;

    debug!("Waiting for transaction confirmation: {}", signature);
    let confirmed = rpc.confirm_transaction(signature).await?;
    if !confirmed {
        return Err(anyhow::anyhow!(
            "Transaction {} failed to confirm for tree {}",
            signature,
            context.merkle_tree
        ));
    }

    info!(
        "Transaction confirmed successfully: {} for tree: {}",
        signature, context.merkle_tree
    );

    Ok(signature.to_string())
}