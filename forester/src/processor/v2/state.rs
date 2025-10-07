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
use light_client::rpc::Rpc;
use light_registry::account_compression_cpi::sdk::{
    create_batch_append_instruction, create_batch_nullify_instruction,
};
use solana_program::instruction::Instruction;
use solana_sdk::signer::Signer;
use tracing::instrument;

use super::common::{process_stream, BatchContext, ParsedMerkleTreeData, ParsedQueueData};
use crate::Result;

async fn create_nullify_stream_future<R>(
    ctx: &BatchContext<R>,
    merkle_tree_data: ParsedMerkleTreeData,
) -> Result<(
    impl Stream<Item = Result<Vec<InstructionDataBatchNullifyInputs>>> + Send,
    u16,
)>
where
    R: Rpc,
{
    let (stream, size) = get_nullify_instruction_stream(
        ctx.rpc_pool.clone(),
        ctx.merkle_tree,
        ctx.prover_update_url.clone(),
        ctx.prover_api_key.clone(),
        ctx.prover_polling_interval,
        ctx.prover_max_wait_time,
        merkle_tree_data,
    )
    .await
    .map_err(Error::from)?;
    let stream = stream.map(|item| item.map_err(Error::from));
    Ok((stream, size))
}

async fn create_append_stream_future<R>(
    ctx: &BatchContext<R>,
    merkle_tree_data: ParsedMerkleTreeData,
    output_queue_data: ParsedQueueData,
) -> Result<(
    impl Stream<Item = Result<Vec<InstructionDataBatchAppendInputs>>> + Send,
    u16,
)>
where
    R: Rpc,
{
    let (stream, size) = get_append_instruction_stream(
        ctx.rpc_pool.clone(),
        ctx.merkle_tree,
        ctx.prover_append_url.clone(),
        ctx.prover_api_key.clone(),
        ctx.prover_polling_interval,
        ctx.prover_max_wait_time,
        merkle_tree_data,
        output_queue_data,
    )
    .await
    .map_err(Error::from)?;
    let stream = stream.map(|item| item.map_err(Error::from));
    Ok((stream, size))
}

#[instrument(
    level = "debug",
    skip(context, merkle_tree_data),
    fields(merkle_tree = ?context.merkle_tree)
)]
pub(crate) async fn perform_nullify<R: Rpc>(
    context: &BatchContext<R>,
    merkle_tree_data: ParsedMerkleTreeData,
) -> Result<()> {
    let instruction_builder = |data: &InstructionDataBatchNullifyInputs| -> Instruction {
        create_batch_nullify_instruction(
            context.authority.pubkey(),
            context.derivation,
            context.merkle_tree,
            context.epoch,
            data.try_to_vec().unwrap(),
        )
    };

    let stream_future = create_nullify_stream_future(context, merkle_tree_data);

    process_stream(context, stream_future, instruction_builder).await?;
    Ok(())
}

#[instrument(
    level = "debug",
    skip(context, merkle_tree_data, output_queue_data),
    fields(merkle_tree = ?context.merkle_tree)
)]
pub(crate) async fn perform_append<R: Rpc>(
    context: &BatchContext<R>,
    merkle_tree_data: ParsedMerkleTreeData,
    output_queue_data: ParsedQueueData,
) -> Result<()> {
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

    let stream_future = create_append_stream_future(context, merkle_tree_data, output_queue_data);
    process_stream(context, stream_future, instruction_builder).await?;
    Ok(())
}
