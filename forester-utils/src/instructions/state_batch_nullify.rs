use std::{pin::Pin, sync::Arc, time::Duration};

use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use async_stream::stream;
use futures::{future, stream::Stream};
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT,
    merkle_tree::{InstructionDataBatchNullifyInputs},
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_hasher::bigint::bigint_to_be_bytes_array;
use light_merkle_tree_metadata::QueueType;
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::batch_update::{get_batch_update_inputs, BatchUpdateCircuitInputs},
};
use tokio::sync::Mutex;
use tracing::{debug, trace};

use crate::{error::ForesterUtilsError, rpc_pool::SolanaRpcPool, utils::wait_for_indexer};

async fn generate_nullify_zkp_proof(
    inputs: BatchUpdateCircuitInputs,
    proof_client: Arc<ProofClient>,
) -> Result<InstructionDataBatchNullifyInputs, ForesterUtilsError> {
    let (proof, new_root) = proof_client
        .generate_batch_update_proof(inputs)
        .await
        .map_err(|e| ForesterUtilsError::Prover(e.to_string()))?;
    Ok(InstructionDataBatchNullifyInputs {
        new_root,
        compressed_proof: CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        },
    })
}

pub async fn get_nullify_instruction_stream<'a, R, I>(
    rpc_pool: Arc<SolanaRpcPool<R>>,
    indexer: Arc<Mutex<I>>,
    merkle_tree_pubkey: Pubkey,
    prover_url: String,
    polling_interval: Duration,
    max_wait_time: Duration,
    merkle_tree_data: crate::ParsedMerkleTreeData,
) -> Result<
    (
        Pin<
            Box<
                dyn Stream<Item = Result<InstructionDataBatchNullifyInputs, ForesterUtilsError>>
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
    let rpc = rpc_pool.get_connection().await?;
    
    let (mut current_root, leaves_hash_chains, num_inserted_zkps, zkp_batch_size) = (
        merkle_tree_data.current_root,
        merkle_tree_data.leaves_hash_chains,
        merkle_tree_data.num_inserted_zkps,
        merkle_tree_data.zkp_batch_size,
    );

    if leaves_hash_chains.is_empty() {
        debug!("No hash chains to process for nullification, returning empty stream.");
        return Ok((Box::pin(futures::stream::empty()), zkp_batch_size));
    }

    let indexer_guard = indexer.lock().await;
    wait_for_indexer(&*rpc, &*indexer_guard).await?;
    drop(rpc);
    drop(indexer_guard);

    let stream = stream! {
        let total_elements = zkp_batch_size as usize * leaves_hash_chains.len();
        let offset = num_inserted_zkps * zkp_batch_size as u64;

        trace!("Requesting {} total elements with offset {}", total_elements, offset);

        let all_queue_elements = {
            let mut indexer_guard = indexer.lock().await;
            indexer_guard
            .get_queue_elements(
                merkle_tree_pubkey.to_bytes(),
                QueueType::InputStateV2,
                total_elements as u16,
                Some(offset),
                None,
            )
            .await
        };

        let all_queue_elements = match all_queue_elements {
            Ok(res) => res.value.items,
            Err(e) => {
                yield Err(ForesterUtilsError::Indexer(format!("Failed to get queue elements: {}", e)));
                return;
            }
        };

        trace!("Got {} queue elements in total", all_queue_elements.len());
        if all_queue_elements.len() != total_elements {
            yield Err(ForesterUtilsError::Indexer(format!(
                "Expected {} elements, got {}",
                total_elements, all_queue_elements.len()
            )));
            return;
        }

        if let Some(first_element) = all_queue_elements.first() {
            if first_element.root != current_root {
                yield Err(ForesterUtilsError::Indexer("Root mismatch between indexer and on-chain state".into()));
                return;
            }
        }

        let mut all_changelogs = Vec::new();
        let proof_client = Arc::new(ProofClient::with_config(prover_url.clone(), polling_interval, max_wait_time));
        let mut proof_futures = Vec::new();

        for (batch_offset, leaves_hash_chain) in leaves_hash_chains.iter().enumerate() {
            let start_idx = batch_offset * zkp_batch_size as usize;
            let end_idx = start_idx + zkp_batch_size as usize;
            let batch_elements = &all_queue_elements[start_idx..end_idx];

            let mut leaves = Vec::new();
            let mut tx_hashes = Vec::new();
            let mut old_leaves = Vec::new();
            let mut path_indices = Vec::new();
            let mut merkle_proofs = Vec::new();

            for leaf_info in batch_elements.iter() {
                path_indices.push(leaf_info.leaf_index as u32);
                leaves.push(leaf_info.account_hash);
                old_leaves.push(leaf_info.leaf);
                merkle_proofs.push(leaf_info.proof.clone());
                tx_hashes.push(leaf_info.tx_hash.ok_or_else(|| ForesterUtilsError::Indexer(format!("Missing tx_hash for leaf index {}", leaf_info.leaf_index)))?);
            }

            let (circuit_inputs, batch_changelog) = match get_batch_update_inputs::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                current_root, tx_hashes, leaves, *leaves_hash_chain, old_leaves, merkle_proofs, path_indices, zkp_batch_size as u32, &all_changelogs,
            ) {
                Ok(inputs) => inputs,
                Err(e) => {
                    yield Err(ForesterUtilsError::Prover(format!("Failed to get batch update inputs: {}", e)));
                    return;
                }
            };

            all_changelogs.extend(batch_changelog);
            current_root = bigint_to_be_bytes_array::<32>(&circuit_inputs.new_root.to_biguint().unwrap()).unwrap();

            let client = Arc::clone(&proof_client);
            proof_futures.push(generate_nullify_zkp_proof(circuit_inputs, client));
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
            yield Err(ForesterUtilsError::Prover(format!("Nullify proof generation failed at batch {}: {}", index, error)));
        }
    };

    Ok((Box::pin(stream), zkp_batch_size))
}
