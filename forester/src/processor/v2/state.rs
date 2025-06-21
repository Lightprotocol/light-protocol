use anyhow::{Error, Ok};
use borsh::BorshSerialize;
use forester_utils::instructions::{
    state_batch_append::get_append_instruction_stream,
    state_batch_nullify::get_nullify_instruction_stream,
};
use futures::stream::{Stream, StreamExt};
use light_batched_merkle_tree::merkle_tree::{
    InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_registry::account_compression_cpi::sdk::{
    create_batch_append_instruction, create_batch_nullify_instruction,
};
use solana_program::instruction::Instruction;
use solana_sdk::signer::Signer;
use tracing::{info, instrument};

use super::common::{process_stream, BatchContext};
use crate::Result;

async fn create_append_stream_future<R, I>(
    ctx: &BatchContext<R, I>,
) -> Result<(
    impl Stream<Item = Result<InstructionDataBatchAppendInputs>> + Send,
    u16,
)>
where
    R: Rpc,
    I: Indexer + 'static,
{
    let (stream, size) = get_append_instruction_stream(
        ctx.rpc_pool.clone(),
        ctx.indexer.clone(),
        ctx.merkle_tree,
        ctx.output_queue,
        ctx.prover_url.clone(),
        ctx.prover_polling_interval,
        ctx.prover_max_wait_time,
    )
    .await
    .map_err(Error::from)?;
    let stream = stream.map(|item| item.map_err(Error::from));
    Ok((stream, size))
}

#[instrument(level = "debug", skip(context))]
pub(crate) async fn perform_append<R: Rpc, I: Indexer + 'static>(
    context: &BatchContext<R, I>,
) -> Result<()> {
    info!(
        "V2_TPS_METRIC: operation_start tree_type=StateV2 operation=append tree={} epoch={}",
        context.merkle_tree, context.epoch
    );
    let instruction_builder = |data: &InstructionDataBatchAppendInputs| -> Instruction {
        create_batch_append_instruction(
            context.authority.pubkey(),
            context.derivation,
            context.merkle_tree,
            context.output_queue,
            context.epoch,
            data.try_to_vec().unwrap(),
        )
    };

    let stream_future = create_append_stream_future(context);
    process_stream(
        context,
        stream_future,
        instruction_builder,
        "StateV2",
        Some("append"),
    )
    .await?;
    Ok(())
}

async fn create_nullify_stream_future<R, I>(
    ctx: &BatchContext<R, I>,
) -> Result<(
    impl Stream<Item = Result<InstructionDataBatchNullifyInputs>> + Send,
    u16,
)>
where
    R: Rpc,
    I: Indexer + 'static,
{
    let (stream, size) = get_nullify_instruction_stream(
        ctx.rpc_pool.clone(),
        ctx.indexer.clone(),
        ctx.merkle_tree,
        ctx.prover_url.clone(),
        ctx.prover_polling_interval,
        ctx.prover_max_wait_time,
    )
    .await
    .map_err(Error::from)?;
    let stream = stream.map(|item| item.map_err(Error::from));
    Ok((stream, size))
}

#[instrument(level = "debug", skip(context))]
pub(crate) async fn perform_nullify<R: Rpc, I: Indexer + 'static>(
    context: &BatchContext<R, I>,
) -> Result<()> {
    info!(
        "V2_TPS_METRIC: operation_start tree_type=StateV2 operation=nullify tree={} epoch={}",
        context.merkle_tree, context.epoch
    );
    // let batch_index = {
    //     let rpc = context.rpc_pool.get_connection().await?;
    //     get_batch_index(context, &*rpc).await?
    // };

    let instruction_builder = |data: &InstructionDataBatchNullifyInputs| -> Instruction {
        create_batch_nullify_instruction(
            context.authority.pubkey(),
            context.derivation,
            context.merkle_tree,
            context.epoch,
            data.try_to_vec().unwrap(),
        )
    };

    let stream_future = create_nullify_stream_future(context);

    process_stream(
        context,
        stream_future,
        instruction_builder,
        "StateV2",
        Some("nullify"),
    )
    .await?;
    Ok(())
}

// async fn get_batch_index<R: Rpc>(
//     context: &BatchContext<R, impl Indexer>,
//     rpc: &R,
// ) -> Result<usize> {
//     let mut account = rpc.get_account(context.merkle_tree).await?.ok_or_else(|| {
//         Error::msg(format!(
//             "State tree account not found: {}",
//             context.merkle_tree
//         ))
//     })?;

//     let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
//         account.data.as_mut_slice(),
//         &context.merkle_tree.into(),
//     )?;

//     Ok(merkle_tree.queue_batches.pending_batch_index as usize)
// }
