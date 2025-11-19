/// Transaction submission logic for batch operations.
use anyhow::Result;
use borsh::BorshSerialize;
use light_batched_merkle_tree::merkle_tree::{
    InstructionDataAddressAppendInputs, InstructionDataBatchAppendInputs,
    InstructionDataBatchNullifyInputs,
};
use light_client::rpc::Rpc;
use light_registry::account_compression_cpi::sdk::{
    create_batch_append_instruction, create_batch_nullify_instruction,
    create_batch_update_address_tree_instruction,
};
use solana_sdk::{instruction::Instruction, signer::Signer};
use tracing::info;

use crate::processor::v2::common::{send_transaction_batch, BatchContext};

const MAX_INSTRUCTIONS_PER_TX: usize = 4;

/// Generic batch submission helper that eliminates code duplication across different batch types.
///
/// Takes a closure that creates instructions from serialized proof data, allowing this function
/// to work with append, nullify, and address batches without duplication.
async fn submit_batches<R, P, F>(
    context: &BatchContext<R>,
    proofs: Vec<P>,
    zkp_batch_size: u16,
    operation_name: &str,
    create_instruction: F,
) -> Result<usize>
where
    R: Rpc,
    P: BorshSerialize,
    F: Fn(Vec<u8>) -> Instruction,
{
    if proofs.is_empty() {
        return Ok(0);
    }

    let total_batches = proofs.len();
    info!("Submitting {} {} batches", total_batches, operation_name);

    let mut total_batches_submitted = 0;

    for (chunk_idx, batch_chunk) in proofs.chunks(MAX_INSTRUCTIONS_PER_TX).enumerate() {
        let start_batch_idx = chunk_idx * MAX_INSTRUCTIONS_PER_TX;

        let mut instructions = Vec::new();
        for batch_data in batch_chunk {
            let serialized = batch_data
                .try_to_vec()
                .map_err(|e| anyhow::anyhow!("Failed to serialize {} proof: {}", operation_name, e))?;
            instructions.push(create_instruction(serialized));
        }

        info!(
            "Submitting {} transaction {} with {} batches (batches {}-{})",
            operation_name,
            chunk_idx,
            instructions.len(),
            start_batch_idx,
            start_batch_idx + instructions.len() - 1
        );

        let signature = send_transaction_batch(context, instructions).await?;

        info!(
            "{} transaction {} submitted: {} ({} batches)",
            operation_name,
            chunk_idx,
            signature,
            batch_chunk.len()
        );

        total_batches_submitted += batch_chunk.len();
    }

    let total_elements = total_batches_submitted * zkp_batch_size as usize;
    let num_transactions = total_batches.div_ceil(MAX_INSTRUCTIONS_PER_TX);

    info!(
        "Submitted {} {} batches ({} elements) in {} transactions",
        total_batches, operation_name, total_elements, num_transactions
    );

    Ok(total_elements)
}

/// Submits append batches to the blockchain.
///
/// Returns the total number of elements processed (not batches).
pub async fn submit_append_batches<R: Rpc>(
    context: &BatchContext<R>,
    proofs: Vec<InstructionDataBatchAppendInputs>,
    zkp_batch_size: u16,
) -> Result<usize> {
    let authority = context.authority.pubkey();
    let derivation = context.derivation;
    let merkle_tree = context.merkle_tree;
    let output_queue = context.output_queue;
    let epoch = context.epoch;

    submit_batches(
        context,
        proofs,
        zkp_batch_size,
        "append",
        move |serialized| {
            create_batch_append_instruction(
                authority,
                derivation,
                merkle_tree,
                output_queue,
                epoch,
                serialized,
            )
        },
    )
    .await
}

/// Submits nullify batches to the blockchain.
///
/// Returns the total number of elements processed (not batches).
pub async fn submit_nullify_batches<R: Rpc>(
    context: &BatchContext<R>,
    proofs: Vec<InstructionDataBatchNullifyInputs>,
    zkp_batch_size: u16,
) -> Result<usize> {
    let authority = context.authority.pubkey();
    let derivation = context.derivation;
    let merkle_tree = context.merkle_tree;
    let epoch = context.epoch;

    submit_batches(
        context,
        proofs,
        zkp_batch_size,
        "nullify",
        move |serialized| {
            create_batch_nullify_instruction(authority, derivation, merkle_tree, epoch, serialized)
        },
    )
    .await
}

pub async fn submit_interleaved_batches<R: Rpc>(
    context: &BatchContext<R>,
    append_proofs: Vec<InstructionDataBatchAppendInputs>,
    append_zkp_batch_size: u16,
    nullify_proofs: Vec<InstructionDataBatchNullifyInputs>,
    nullify_zkp_batch_size: u16,
    pattern: &[super::types::BatchType],
) -> Result<usize> {
    debug_assert!(
        append_proofs.is_empty() || append_zkp_batch_size > 0,
        "append_zkp_batch_size must be > 0 when append proofs exist"
    );
    debug_assert!(
        nullify_proofs.is_empty() || nullify_zkp_batch_size > 0,
        "nullify_zkp_batch_size must be > 0 when nullify proofs exist"
    );
    use super::types::BatchType;

    info!(
        "Submitting {} append + {} nullify batches in interleaved pattern",
        append_proofs.len(),
        nullify_proofs.len()
    );

    let mut current_tx_instructions = Vec::new();
    let mut append_idx = 0;
    let mut nullify_idx = 0;
    let mut submitted_append_batches = 0usize;
    let mut submitted_nullify_batches = 0usize;

    for batch_type in pattern.iter() {
        let instruction = match batch_type {
            BatchType::Append if append_idx < append_proofs.len() => {
                let proof = &append_proofs[append_idx];
                submitted_append_batches += 1;
                append_idx += 1;
                create_batch_append_instruction(
                    context.authority.pubkey(),
                    context.derivation,
                    context.merkle_tree,
                    context.output_queue,
                    context.epoch,
                    proof.try_to_vec()?,
                )
            }
            BatchType::Nullify if nullify_idx < nullify_proofs.len() => {
                let proof = &nullify_proofs[nullify_idx];
                submitted_nullify_batches += 1;
                nullify_idx += 1;
                create_batch_nullify_instruction(
                    context.authority.pubkey(),
                    context.derivation,
                    context.merkle_tree,
                    context.epoch,
                    proof.try_to_vec()?,
                )
            }
            _ => continue,
        };

        current_tx_instructions.push(instruction);

        if current_tx_instructions.len() == MAX_INSTRUCTIONS_PER_TX
            || (append_idx + nullify_idx) == (append_proofs.len() + nullify_proofs.len())
        {
            let signature =
                send_transaction_batch(context, current_tx_instructions.clone()).await?;
            info!(
                "Submitted interleaved TX with {} batches (signature: {})",
                current_tx_instructions.len(),
                signature
            );

            current_tx_instructions.clear();
        }
    }

    info!(
        "Submitted {} append batches and {} nullify batches in interleaved pattern",
        submitted_append_batches, submitted_nullify_batches
    );

    let total_elements = submitted_append_batches * append_zkp_batch_size as usize
        + submitted_nullify_batches * nullify_zkp_batch_size as usize;

    Ok(total_elements)
}

/// Submits address append batches to the blockchain.
///
/// Returns the total number of elements processed (not batches).
pub async fn submit_address_batches<R: Rpc>(
    context: &BatchContext<R>,
    proofs: Vec<InstructionDataAddressAppendInputs>,
    zkp_batch_size: u16,
) -> Result<usize> {
    let authority = context.authority.pubkey();
    let derivation = context.derivation;
    let merkle_tree = context.merkle_tree;
    let epoch = context.epoch;

    submit_batches(
        context,
        proofs,
        zkp_batch_size,
        "address append",
        move |serialized| {
            create_batch_update_address_tree_instruction(
                authority,
                derivation,
                merkle_tree,
                epoch,
                serialized,
            )
        },
    )
    .await
}
