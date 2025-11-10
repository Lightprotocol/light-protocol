use std::{pin::Pin, sync::Arc, time::Duration};

use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use async_stream::stream;
use futures::stream::Stream;
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT, merkle_tree::InstructionDataBatchNullifyInputs,
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_hasher::bigint::bigint_to_be_bytes_array;
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::batch_update::{get_batch_update_inputs, BatchUpdateCircuitInputs},
};
use tracing::{debug, warn};

use crate::{
    error::ForesterUtilsError, rpc_pool::SolanaRpcPool, utils::wait_for_indexer,
    ParsedMerkleTreeData,
};

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

#[allow(clippy::too_many_arguments)]
pub async fn get_nullify_instruction_stream<'a, R: Rpc>(
    rpc_pool: Arc<SolanaRpcPool<R>>,
    merkle_tree_pubkey: Pubkey,
    prover_url: String,
    prover_api_key: Option<String>,
    polling_interval: Duration,
    max_wait_time: Duration,
    merkle_tree_data: ParsedMerkleTreeData,
) -> Result<
    (
        Pin<
            Box<
                dyn Stream<
                        Item = Result<Vec<InstructionDataBatchNullifyInputs>, ForesterUtilsError>,
                    > + Send
                    + 'a,
            >,
        >,
        u16,
    ),
    ForesterUtilsError,
> {
    let (mut current_root, leaves_hash_chains, _num_inserted_zkps, zkp_batch_size) = (
        merkle_tree_data.current_root,
        merkle_tree_data.leaves_hash_chains,
        merkle_tree_data.num_inserted_zkps,
        merkle_tree_data.zkp_batch_size,
    );

    if leaves_hash_chains.is_empty() {
        debug!("No hash chains to process for nullification, returning empty stream.");
        return Ok((Box::pin(futures::stream::empty()), zkp_batch_size));
    }

    let rpc = rpc_pool.get_connection().await?;
    wait_for_indexer(&*rpc).await?;
    drop(rpc);

    let stream = stream! {
        let mut next_queue_index: Option<u64> = None;
        let mut all_changelogs = Vec::new();
        let proof_client = Arc::new(ProofClient::with_config(prover_url.clone(), polling_interval, max_wait_time, prover_api_key.clone()));

        let mut expected_indexer_root = current_root;
        let mut proofs_buffer = Vec::new();
        const MAX_PROOFS_PER_TX: usize = 3;  // Bundle up to 3 proofs per transaction

        for (batch_idx, leaves_hash_chain) in leaves_hash_chains.iter().enumerate() {
            debug!(
                "Fetching batch {} - tree: {}, start_queue_index: {:?}, limit: {}",
                batch_idx, merkle_tree_pubkey, next_queue_index, zkp_batch_size
            );

            if !proofs_buffer.is_empty() && batch_idx > 0 {
                debug!("Sending {} accumulated proofs before fetching batch {}", proofs_buffer.len(), batch_idx);
                yield Ok(proofs_buffer.clone());
                proofs_buffer.clear();
                debug!("Waiting for transaction to land and indexer to sync...");
                let rpc = rpc_pool.get_connection().await?;
                if let Err(e) = wait_for_indexer(&*rpc).await {
                    yield Err(ForesterUtilsError::Indexer(format!("Failed to wait for indexer sync after transaction: {}", e)));
                    return;
                }
                drop(rpc);
                expected_indexer_root = current_root;
                debug!("Transaction landed, updated expected root for batch {}", batch_idx);
            }

            let queue_elements_result = {
                let mut connection = rpc_pool.get_connection().await?;
                let indexer = connection.indexer_mut()?;
                indexer.get_queue_elements(
                    merkle_tree_pubkey.to_bytes(),
                    None,
                    None,
                    next_queue_index,
                    Some(zkp_batch_size),
                    None,
                )
                .await
            };

            let (batch_elements, batch_first_queue_idx) = match queue_elements_result {
                Ok(res) => {
                    let items = res.value.input_queue_elements.unwrap_or_default();
                    let first_idx = res.value.input_queue_index;
                    if items.len() != zkp_batch_size as usize {
                        warn!(
                            "Got {} elements but expected {}, stopping",
                            items.len(), zkp_batch_size
                        );
                        break;
                    }

                    (items, first_idx)
                },
                Err(e) => {
                    yield Err(ForesterUtilsError::Indexer(format!("Failed to get queue elements for batch {}: {}", batch_idx, e)));
                    return;
                }
            };

            if let Some(first_element) = batch_elements.first() {
                if first_element.root != expected_indexer_root {
                    debug!(
                        "Root mismatch for batch {}: indexer root {:?} != expected root {:?}",
                        batch_idx, first_element.root, expected_indexer_root
                    );
                    yield Err(ForesterUtilsError::Indexer("Root mismatch between indexer and expected state".into()));
                    return;
                }
            }

            if let Some(first_idx) = batch_first_queue_idx {
                next_queue_index = Some(first_idx + zkp_batch_size as u64);
                debug!("Next batch will start at queue index: {}", first_idx + zkp_batch_size as u64);
            }

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
            match generate_nullify_zkp_proof(circuit_inputs, client).await {
                Ok(proof) => {
                    debug!("Generated proof for batch {}", batch_idx);
                    proofs_buffer.push(proof);

                    let should_send = if proofs_buffer.len() >= MAX_PROOFS_PER_TX {
                        debug!("Buffer full with {} proofs, sending transaction", proofs_buffer.len());
                        true
                    } else {
                        false
                    };

                    if should_send {
                        debug!("Yielding {} proofs for transaction", proofs_buffer.len());
                        yield Ok(proofs_buffer.clone());
                        proofs_buffer.clear();

                        if batch_idx < leaves_hash_chains.len() - 1 {
                            debug!("Waiting for transaction to land before continuing...");
                            let rpc = rpc_pool.get_connection().await?;
                            if let Err(e) = wait_for_indexer(&*rpc).await {
                                yield Err(ForesterUtilsError::Indexer(format!("Failed to wait for indexer sync: {}", e)));
                                return;
                            }
                            drop(rpc);
                            expected_indexer_root = current_root;
                            debug!("Transaction landed, continuing with next batches");
                        }
                    }
                },
                Err(e) => {
                    yield Err(e);
                    return;
                }
            }
        }

        if !proofs_buffer.is_empty() {
            debug!("Sending final {} proofs", proofs_buffer.len());
            yield Ok(proofs_buffer);
        }
    };

    Ok((Box::pin(stream), zkp_batch_size))
}
