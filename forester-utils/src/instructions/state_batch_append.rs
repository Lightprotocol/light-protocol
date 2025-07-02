use std::{pin::Pin, sync::Arc, time::Duration};

use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use async_stream::stream;
use futures::{future, stream::Stream};
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT,
    merkle_tree::{InstructionDataBatchAppendInputs},
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_hasher::bigint::bigint_to_be_bytes_array;
use light_merkle_tree_metadata::QueueType;
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::batch_append::{get_batch_append_inputs, BatchAppendsCircuitInputs},
};
use light_sparse_merkle_tree::changelog::ChangelogEntry;
use tokio::sync::Mutex;
use tracing::trace;

use crate::{error::ForesterUtilsError, rpc_pool::SolanaRpcPool, utils::wait_for_indexer};

async fn generate_zkp_proof(
    circuit_inputs: BatchAppendsCircuitInputs,
    proof_client: Arc<ProofClient>,
) -> Result<InstructionDataBatchAppendInputs, ForesterUtilsError> {
    let (proof, new_root) = proof_client
        .generate_batch_append_proof(circuit_inputs)
        .await
        .map_err(|e| ForesterUtilsError::Prover(e.to_string()))?;
    Ok(InstructionDataBatchAppendInputs {
        new_root,
        compressed_proof: CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        },
    })
}

pub async fn get_append_instruction_stream<'a, R, I>(
    rpc_pool: Arc<SolanaRpcPool<R>>,
    indexer: Arc<Mutex<I>>,
    merkle_tree_pubkey: Pubkey,
    prover_url: String,
    polling_interval: Duration,
    max_wait_time: Duration,
    merkle_tree_data: crate::ParsedMerkleTreeData,
    output_queue_data: crate::ParsedQueueData,
) -> Result<
    (
        Pin<
            Box<
                dyn Stream<Item = Result<InstructionDataBatchAppendInputs, ForesterUtilsError>>
                    + Send
                    + 'a,
            >,
        >,
        u16,
    ),
    ForesterUtilsError,
>
where
    R: Rpc + Send + Sync + 'a,
    I: Indexer + Send + 'a,
{
    trace!("Initializing append batch instruction stream with parsed data");

    let (indexer_guard, rpc_result) = tokio::join!(indexer.lock(), rpc_pool.get_connection());
    let rpc = rpc_result?;

    let (merkle_tree_next_index, mut current_root, _,) = (merkle_tree_data.next_index, merkle_tree_data.current_root, merkle_tree_data.root_history);
    let (zkp_batch_size, leaves_hash_chains)= (output_queue_data.zkp_batch_size, output_queue_data.leaves_hash_chains);

    if leaves_hash_chains.is_empty() {
        trace!("No hash chains to process, returning empty stream.");
        return Ok((Box::pin(futures::stream::empty()), zkp_batch_size));
    }

    wait_for_indexer(&*rpc, &*indexer_guard).await?;
    drop(rpc);
    drop(indexer_guard);

    let stream = stream! {
        let total_elements = zkp_batch_size as usize * leaves_hash_chains.len();
        let offset = merkle_tree_next_index;

        let queue_elements = {
            let mut indexer_guard = indexer.lock().await;

            match indexer_guard
                .get_queue_elements(
                    merkle_tree_pubkey.to_bytes(),
                    QueueType::OutputStateV2,
                    total_elements as u16,
                    Some(offset),
                    None,
                )
                .await {
                    Ok(res) => res.value.items,
                    Err(e) => {
                        yield Err(ForesterUtilsError::Indexer(format!("Failed to get queue elements: {}", e)));
                        return;
                    }
                }
        };

        if queue_elements.len() != total_elements {
            yield Err(ForesterUtilsError::Indexer(format!(
                "Expected {} elements, got {}",
                total_elements,
                queue_elements.len()
            )));
            return;
        }

        if let Some(first_element) = queue_elements.first() {
            if first_element.root != current_root {
                 yield Err(ForesterUtilsError::Indexer("Root mismatch between indexer and on-chain state".into()));
                 return;
            }
        }

        let mut all_changelogs: Vec<ChangelogEntry<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>> = Vec::new();
        let proof_client = Arc::new(ProofClient::with_config(prover_url.clone(), polling_interval, max_wait_time));
        let mut proof_futures = Vec::new();

        for (batch_idx, leaves_hash_chain) in leaves_hash_chains.iter().enumerate() {
            let start_idx = batch_idx * zkp_batch_size as usize;
            let end_idx = start_idx + zkp_batch_size as usize;
            let batch_elements = &queue_elements[start_idx..end_idx];

            let old_leaves: Vec<[u8; 32]> = batch_elements.iter().map(|x| x.leaf).collect();
            let leaves: Vec<[u8; 32]> = batch_elements.iter().map(|x| x.account_hash).collect();
            let merkle_proofs: Vec<Vec<[u8; 32]>> = batch_elements.iter().map(|x| x.proof.clone()).collect();
            let adjusted_start_index = merkle_tree_next_index as u32 + (batch_idx * zkp_batch_size as usize) as u32;

            let (circuit_inputs, batch_changelogs) = match get_batch_append_inputs::<32>(
                current_root, adjusted_start_index, leaves, *leaves_hash_chain, old_leaves, merkle_proofs, zkp_batch_size as u32, &all_changelogs,
            ) {
                Ok(inputs) => inputs,
                Err(e) => {
                    yield Err(ForesterUtilsError::Prover(format!("Failed to get circuit inputs: {}", e)));
                    return;
                }
            };

            current_root = bigint_to_be_bytes_array::<32>(&circuit_inputs.new_root.to_biguint().unwrap()).unwrap();
            all_changelogs.extend(batch_changelogs);

            let client = Arc::clone(&proof_client);
            proof_futures.push(generate_zkp_proof(circuit_inputs, client));
        }

        let proof_results = future::join_all(proof_futures).await;

        let mut successful_proofs = Vec::new();
        let mut first_error = None;

        for (index, proof_result) in proof_results.into_iter().enumerate() {
            match proof_result {
                Ok(data) => {
                    if first_error.is_none() {
                        successful_proofs.push(data);
                    }
                },
                Err(e) => {
                    if first_error.is_none() {
                        first_error = Some((index, e));
                    }
                }
            }
        }

        for proof in successful_proofs {
            yield Ok(proof);
        }

        if let Some((index, error)) = first_error {
            yield Err(ForesterUtilsError::Prover(format!("Proof generation failed at batch {}: {}", index, error)));
        }
    };

    Ok((Box::pin(stream), zkp_batch_size))
}
