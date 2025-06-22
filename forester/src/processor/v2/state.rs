use anyhow::{Error, Ok};
use borsh::BorshSerialize;
use forester_utils::instructions::{
    state_batch_append::get_append_instruction_stream,
    state_batch_nullify::get_nullify_instruction_stream,
};
use futures::stream::{Stream, StreamExt};
use light_batched_merkle_tree::merkle_tree::{
    BatchedMerkleTreeAccount, InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_registry::account_compression_cpi::sdk::{
    create_batch_append_instruction, create_batch_nullify_instruction,
};
use solana_program::instruction::Instruction;
use solana_sdk::signer::Signer;
use tracing::instrument;

use super::common::{process_stream, BatchContext};
use crate::{
    indexer_type::{
        update_test_indexer_after_append, update_test_indexer_after_nullification, IndexerType,
    },
    Result,
};

async fn create_append_stream_future<R, I>(
    ctx: &BatchContext<R, I>,
) -> Result<(
    impl Stream<Item = Result<InstructionDataBatchAppendInputs>> + Send,
    u16,
)>
where
    R: Rpc,
    I: Indexer + IndexerType<R> + 'static,
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

async fn create_append_finalizer_future<R, I>(ctx: &BatchContext<R, I>) -> Result<()>
where
    R: Rpc,
    I: Indexer + IndexerType<R> + 'static,
{
    let mut rpc = ctx.rpc_pool.get_connection().await?;
    update_test_indexer_after_append(
        &mut *rpc,
        ctx.indexer.clone(),
        ctx.merkle_tree,
        ctx.output_queue,
    )
    .await
    .map_err(Error::from)
}

#[instrument(level = "debug", skip(context))]
pub(crate) async fn perform_append<R: Rpc, I: Indexer + IndexerType<R> + 'static>(
    context: &BatchContext<R, I>,
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

    let stream_future = create_append_stream_future(context);
    let finalizer_future = create_append_finalizer_future(context);

    process_stream(
        context,
        stream_future,
        instruction_builder,
        finalizer_future,
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
    I: Indexer + IndexerType<R> + 'static,
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

async fn create_nullify_finalizer_future<R, I>(
    ctx: &BatchContext<R, I>,
    batch_index: usize,
) -> Result<()>
where
    R: Rpc,
    I: Indexer + IndexerType<R> + 'static,
{
    let mut rpc = ctx.rpc_pool.get_connection().await?;
    update_test_indexer_after_nullification(
        &mut *rpc,
        ctx.indexer.clone(),
        ctx.merkle_tree,
        batch_index,
    )
    .await
    .map_err(Error::from)
}

#[instrument(level = "debug", skip(context))]
pub(crate) async fn perform_nullify<R: Rpc, I: Indexer + IndexerType<R> + 'static>(
    context: &BatchContext<R, I>,
) -> Result<()> {
    let batch_index = {
        let rpc = context.rpc_pool.get_connection().await?;
        get_batch_index(context, &*rpc).await?
    };

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
    let finalizer_future = create_nullify_finalizer_future(context, batch_index);

    process_stream(
        context,
        stream_future,
        instruction_builder,
        finalizer_future,
    )
    .await?;
    Ok(())
}

async fn get_batch_index<R: Rpc>(
    context: &BatchContext<R, impl Indexer>,
    rpc: &R,
) -> Result<usize> {
    let mut account = rpc.get_account(context.merkle_tree).await?.ok_or_else(|| {
        Error::msg(format!(
            "State tree account not found: {}",
            context.merkle_tree
        ))
    })?;

    let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
        account.data.as_mut_slice(),
        &context.merkle_tree.into(),
    )?;

    Ok(merkle_tree.queue_batches.pending_batch_index as usize)
}
