use anyhow::Error;
use borsh::BorshSerialize;
use forester_utils::instructions::address_batch_update::{
    get_address_update_instruction_stream, AddressUpdateConfig,
};
use futures::stream::{Stream, StreamExt};
use light_batched_merkle_tree::merkle_tree::InstructionDataAddressAppendInputs;
use light_client::rpc::Rpc;
use light_registry::account_compression_cpi::sdk::create_batch_update_address_tree_instruction;
use solana_program::instruction::Instruction;
use solana_sdk::signer::Signer;
use tracing::instrument;

use super::common::{process_stream, BatchContext, ParsedMerkleTreeData};
use crate::Result;

async fn create_stream_future<R>(
    ctx: &BatchContext<R>,
    merkle_tree_data: ParsedMerkleTreeData,
) -> Result<(
    impl Stream<Item = Result<Vec<InstructionDataAddressAppendInputs>>> + Send,
    u16,
)>
where
    R: Rpc,
{
    let config = AddressUpdateConfig {
        rpc_pool: ctx.rpc_pool.clone(),
        merkle_tree_pubkey: ctx.merkle_tree,
        prover_url: ctx.prover_address_append_url.clone(),
        prover_api_key: ctx.prover_api_key.clone(),
        polling_interval: ctx.prover_polling_interval,
        max_wait_time: ctx.prover_max_wait_time,
    };
    let (stream, size) = get_address_update_instruction_stream(config, merkle_tree_data)
        .await
        .map_err(Error::from)?;
    let stream = stream.map(|item| item.map_err(Error::from));
    Ok((stream, size))
}

#[instrument(level = "debug", skip(context, merkle_tree_data), fields(tree = %context.merkle_tree))]
pub(crate) async fn process_batch<R: Rpc>(
    context: &BatchContext<R>,
    merkle_tree_data: ParsedMerkleTreeData,
) -> Result<usize> {
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
    process_stream(context, stream_future, instruction_builder).await
}
