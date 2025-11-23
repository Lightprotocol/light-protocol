use anyhow::Error;
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
use forester_utils::utils::wait_for_indexer;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use std::time::Duration;
use tokio::time::sleep;

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
                        if let Some(last_root) = append_proofs.last().map(|p| p.new_root) {
                            wait_for_root_and_indexer(context, last_root, &sig).await?;
                        }
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
                        if let Some(last_root) = nullify_proofs.last().map(|p| p.new_root) {
                            wait_for_root_and_indexer(context, last_root, &sig).await?;
                        }
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

async fn wait_for_root_and_indexer<R: Rpc>(
    context: &BatchContext<R>,
    expected_root: [u8; 32],
    sig: &str,
) -> Result<()> {
    // First wait for the indexer to catch up to the RPC slot so we don't refetch stale queues.
    {
        let rpc = context.rpc_pool.get_connection().await?;
        wait_for_indexer(&*rpc).await?;
    }

    const MAX_ATTEMPTS: usize = 20;
    for attempt in 0..MAX_ATTEMPTS {
        let rpc = context.rpc_pool.get_connection().await?;
        if let Some(account) = rpc.get_account(context.merkle_tree).await? {
            let mut data = account.data.clone();
            match BatchedMerkleTreeAccount::state_from_bytes(
                &mut data,
                &context.merkle_tree.into(),
            ) {
                Ok(tree) => {
                    if let Some(root) = tree.get_root() {
                        if root == expected_root {
                            debug!(
                                "Root {:?}[..4] observed on-chain after tx {} (attempt {})",
                                &root[..4],
                                sig,
                                attempt
                            );
                            return Ok(());
                        }
                    }
                }
                Err(_) => {}
            }
        }
        sleep(Duration::from_millis(300)).await;
    }

    Err(anyhow::anyhow!(
        "Merkle tree root did not advance to expected value after tx {}",
        sig
    ))
}
