use anyhow::{Error, Ok};
use borsh::BorshSerialize;
use forester_utils::instructions::state::{get_state_update_instruction_stream, BatchInstruction};
use futures::stream::StreamExt;
use light_client::rpc::Rpc;
use light_registry::account_compression_cpi::sdk::{
    create_batch_append_instruction, create_batch_nullify_instruction,
};
use solana_program::instruction::Instruction;
use solana_sdk::signer::Signer;
use tracing::{debug, instrument};

use super::common::{send_transaction_batch, BatchContext, ParsedMerkleTreeData, ParsedQueueData};
use crate::{errors::ForesterError, Result};

#[instrument(
    level = "debug",
    skip(context, merkle_tree_data, output_queue_data),
    fields(merkle_tree = ?context.merkle_tree)
)]
pub(crate) async fn perform_state_update<R: Rpc>(
    context: &BatchContext<R>,
    merkle_tree_data: ParsedMerkleTreeData,
    output_queue_data: Option<ParsedQueueData>,
) -> Result<()> {
    let (stream, _zkp_batch_size) = get_state_update_instruction_stream(
        context.rpc_pool.clone(),
        context.merkle_tree,
        context.prover_append_url.clone(),
        context.prover_update_url.clone(),
        context.prover_api_key.clone(),
        context.prover_polling_interval,
        context.prover_max_wait_time,
        merkle_tree_data,
        output_queue_data,
        context.staging_tree_cache.clone(),
    )
    .await
    .map_err(Error::from)?;

    let mut stream = Box::pin(stream.map(|item| item.map_err(Error::from)));

    while let Some(result) = stream.next().await {
        let instruction_batch = result?;

        match instruction_batch {
            BatchInstruction::Append(append_proofs) => {
                debug!("append proofs {}", append_proofs.len());
                let instructions: Vec<Instruction> = append_proofs
                    .iter()
                    .map(|data| {
                        create_batch_append_instruction(
                            context.authority.pubkey(),
                            context.derivation,
                            context.merkle_tree,
                            context.output_queue,
                            context.epoch,
                            data.try_to_vec().unwrap(),
                        )
                    })
                    .collect();

                match send_transaction_batch(context, instructions).await {
                    std::result::Result::Ok(sig) => {
                        debug!("append tx {}", sig);
                    }
                    std::result::Result::Err(e) => {
                        if let Some(ForesterError::NotInActivePhase) =
                            e.downcast_ref::<ForesterError>()
                        {
                            break;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
            BatchInstruction::Nullify(nullify_proofs) => {
                debug!("nullify proofs {}", nullify_proofs.len());
                let instructions: Vec<Instruction> = nullify_proofs
                    .iter()
                    .map(|data| {
                        create_batch_nullify_instruction(
                            context.authority.pubkey(),
                            context.derivation,
                            context.merkle_tree,
                            context.epoch,
                            data.try_to_vec().unwrap(),
                        )
                    })
                    .collect();

                match send_transaction_batch(context, instructions).await {
                    std::result::Result::Ok(sig) => {
                        debug!("nullify tx {}", sig);
                    }
                    std::result::Result::Err(e) => {
                        if let Some(ForesterError::NotInActivePhase) =
                            e.downcast_ref::<ForesterError>()
                        {
                            break;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
