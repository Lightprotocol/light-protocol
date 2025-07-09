use anyhow::Error;
use borsh::BorshSerialize;
use forester_utils::instructions::address_batch_update::{
    get_address_update_instruction_stream, AddressUpdateConfig,
};
use futures::stream::{Stream, StreamExt};
use light_batched_merkle_tree::merkle_tree::InstructionDataAddressAppendInputs;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_registry::account_compression_cpi::sdk::create_batch_update_address_tree_instruction;
use solana_program::instruction::Instruction;
use solana_sdk::signer::Signer;
use tracing::{info, instrument};

use super::common::{process_stream, BatchContext, ParsedMerkleTreeData};
use crate::Result;

async fn create_stream_future<R, I>(
    ctx: &BatchContext<R, I>,
    merkle_tree_data: ParsedMerkleTreeData,
) -> Result<(
    impl Stream<Item = Result<Vec<InstructionDataAddressAppendInputs>>> + Send,
    u16,
)>
where
    R: Rpc,
    I: Indexer + 'static,
{
    let config = AddressUpdateConfig {
        rpc_pool: ctx.rpc_pool.clone(),
        indexer: ctx.indexer.clone(),
        merkle_tree_pubkey: ctx.merkle_tree,
        prover_url: ctx.prover_url.clone(),
        polling_interval: ctx.prover_polling_interval,
        max_wait_time: ctx.prover_max_wait_time,
        ixs_per_tx: ctx.ixs_per_tx,
    };
    let (stream, size) = get_address_update_instruction_stream(config, merkle_tree_data)
        .await
        .map_err(Error::from)?;
    let stream = stream.map(|item| item.map_err(Error::from));
    Ok((stream, size))
}

#[instrument(level = "debug", skip(context), fields(tree = %context.merkle_tree))]
pub(crate) async fn process_batch<R: Rpc, I: Indexer + 'static>(
    context: &BatchContext<R, I>,
    merkle_tree_data: ParsedMerkleTreeData,
) -> Result<usize> {
    info!(
        "V2_TPS_METRIC: operation_start tree_type=AddressV2 tree={} epoch={}",
        context.merkle_tree, context.epoch
    );
    let instruction_builder = |data: &InstructionDataAddressAppendInputs| -> Instruction {
        let serialized_data = data.try_to_vec().unwrap();
        create_batch_update_address_tree_instruction(
            context.authority.pubkey(),
            context.derivation,
            context.merkle_tree,
            context.epoch,
            serialized_data,
        )
    };

    let stream_future = create_stream_future(context, merkle_tree_data);
    process_stream(
        context,
        stream_future,
        instruction_builder,
        "AddressV2",
        None,
    )
    .await
}
