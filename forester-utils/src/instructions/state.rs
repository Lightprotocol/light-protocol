use std::{pin::Pin, sync::Arc, time::Duration};

use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use async_stream::stream;
use futures::stream::Stream;
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT,
    merkle_tree::{InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs},
};
use light_client::indexer::QueueElementsV2Options;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_hasher::{Hasher, Poseidon};
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::{
        batch_append::get_batch_append_inputs_v2, batch_update::get_batch_update_inputs_v2,
    },
};
use tokio::sync::Mutex;
use tracing::{debug, error, warn};

use crate::{
    error::ForesterUtilsError, rpc_pool::SolanaRpcPool, staging_tree::StagingTree,
    ParsedMerkleTreeData, ParsedQueueData,
};

const MAX_PROOFS_PER_TX: usize = 3;

#[derive(Debug)]
pub enum BatchInstruction {
    Append(Vec<InstructionDataBatchAppendInputs>),
    Nullify(Vec<InstructionDataBatchNullifyInputs>),
}

#[allow(clippy::too_many_arguments)]
pub async fn get_state_update_instruction_stream<'a, R: Rpc>(
    rpc_pool: Arc<SolanaRpcPool<R>>,
    merkle_tree_pubkey: Pubkey,
    prover_append_url: String,
    prover_update_url: String,
    prover_api_key: Option<String>,
    polling_interval: Duration,
    max_wait_time: Duration,
    merkle_tree_data: ParsedMerkleTreeData,
    output_queue_data: Option<ParsedQueueData>,
    staging_tree_cache: Arc<Mutex<Option<StagingTree>>>,
) -> Result<
    (
        Pin<Box<dyn Stream<Item = Result<BatchInstruction, ForesterUtilsError>> + Send + 'a>>,
        u16,
    ),
    ForesterUtilsError,
> {
    let (merkle_tree_next_index, current_root, _) = (
        merkle_tree_data.next_index,
        merkle_tree_data.current_root,
        merkle_tree_data.root_history,
    );

    let append_hash_chains = output_queue_data
        .as_ref()
        .map(|q| q.leaves_hash_chains.clone())
        .unwrap_or_default();
    let nullify_hash_chains = merkle_tree_data.leaves_hash_chains.clone();

    let zkp_batch_size = output_queue_data
        .as_ref()
        .map(|q| q.zkp_batch_size)
        .unwrap_or(merkle_tree_data.zkp_batch_size);

    if append_hash_chains.is_empty() && nullify_hash_chains.is_empty() {
        return Ok((Box::pin(futures::stream::empty()), zkp_batch_size));
    }

    let stream = stream! {
        let mut staging_tree: Option<StagingTree> = {
            let cache = staging_tree_cache.lock().await;
            if let Some(cached_tree) = cache.as_ref() {
                if cached_tree.current_root() == current_root {
                    Some(cached_tree.clone())
                } else {
                    None
                }
            } else {
                None
            }
        };
        let mut expected_indexer_root = staging_tree
            .as_ref()
            .map(|t| t.current_root())
            .unwrap_or(current_root);

        let append_proof_client = Arc::new(ProofClient::with_config(
            prover_append_url.clone(),
            polling_interval,
            max_wait_time,
            prover_api_key.clone(),
        ));

        let nullify_proof_client = Arc::new(ProofClient::with_config(
            prover_update_url.clone(),
            polling_interval,
            max_wait_time,
            prover_api_key.clone(),
        ));

        let mut next_append_queue_index: Option<u64> = None;
        let mut next_nullify_queue_index: Option<u64> = None;

        let mut prefetched_input_queue: Option<light_client::indexer::InputQueueDataV2> = None;

        let mut proofs_buffer = Vec::new();

        for (batch_idx, leaves_hash_chain) in append_hash_chains.iter().enumerate() {
            if !proofs_buffer.is_empty() && batch_idx > 0 {
                yield Ok(BatchInstruction::Append(proofs_buffer.clone()));
                proofs_buffer.clear();
            }

            let queue_elements_result = {
                let mut connection = rpc_pool.get_connection().await?;
                let indexer = connection.indexer_mut()?;

                let mut options = QueueElementsV2Options::default()
                    .with_output_queue(next_append_queue_index, Some(zkp_batch_size))
                    .with_output_queue_batch_size(Some(zkp_batch_size));

                if !nullify_hash_chains.is_empty() && batch_idx == 0 {
                    options = options
                        .with_input_queue(next_nullify_queue_index, Some(zkp_batch_size))
                        .with_input_queue_batch_size(Some(zkp_batch_size));
                    debug!("Fetching both output and input queue in single V2 call (batch {})", batch_idx);
                }

                indexer
                    .get_queue_elements_v2(
                        merkle_tree_pubkey.to_bytes(),
                        options,
                        None,
                    )
                    .await
            };

            let batch_data = match queue_elements_result {
                Ok(res) => {
                    if batch_idx == 0 && res.value.input_queue.is_some() {
                        debug!("Cached prefetched input queue data for NULLIFY phase");
                        prefetched_input_queue = res.value.input_queue;
                    }

                    let output_queue = res.value.output_queue.ok_or_else(|| {
                        ForesterUtilsError::Indexer("No output queue data in V2 response".into())
                    })?;

                    if output_queue.leaf_indices.len() != zkp_batch_size as usize {
                        warn!(
                            "Got {} elements but expected {}, stopping APPEND phase",
                            output_queue.leaf_indices.len(), zkp_batch_size
                        );
                        break;
                    }

                    output_queue
                },
                Err(e) => {
                    yield Err(ForesterUtilsError::Indexer(format!("Failed to get queue elements for APPEND batch {}: {}", batch_idx, e)));
                    return;
                }
            };

            if batch_data.initial_root != expected_indexer_root {
                error!(
                    "Root mismatch! Indexer root: {:?}, Expected root: {:?}",
                    batch_data.initial_root,
                    expected_indexer_root
                );
                {
                    let mut cache = staging_tree_cache.lock().await;
                    if cache.is_some() {
                        *cache = None;
                    }
                }
                yield Err(ForesterUtilsError::Indexer("Root mismatch between indexer and expected state".into()));
                return;
            }

            next_append_queue_index = Some(batch_data.first_queue_index + zkp_batch_size as u64);

            if staging_tree.is_none() {
                match StagingTree::from_v2_output_queue(
                    &batch_data.leaf_indices,
                    &batch_data.old_leaves,
                    &batch_data.nodes,
                    &batch_data.node_hashes,
                ) {
                    Ok(tree) => {
                        staging_tree = Some(tree);
                    },
                    Err(e) => {
                        yield Err(ForesterUtilsError::Prover(format!("Failed to initialize staging tree: {}", e)));
                        return;
                    }
                }
            }

            let staging = staging_tree.as_mut().unwrap();
            let leaves: Vec<[u8; 32]> = batch_data.account_hashes.clone();

            let (old_leaves, merkle_proofs, old_root, new_root) = match staging.process_batch_updates(
                &batch_data.leaf_indices,
                &leaves,
                "APPEND",
                batch_idx,
            ) {
                Ok(result) => result,
                Err(e) => {
                    yield Err(ForesterUtilsError::StagingTree(format!("Failed to process APPEND batch {}: {}", batch_idx, e)));
                    return;
                }
            };
            let adjusted_start_index = merkle_tree_next_index as u32 + (batch_idx * zkp_batch_size as usize) as u32;

            debug!("Using start_index: {} (min leaf_index from batch)", adjusted_start_index);

            use light_hasher::hash_chain::create_hash_chain_from_slice;
            let indexer_hashchain = create_hash_chain_from_slice(&leaves)
                .map_err(|e| ForesterUtilsError::Prover(format!("Failed to calculate hashchain: {}", e)))?;

            if indexer_hashchain != *leaves_hash_chain {
                error!("Hashchain mismatch! On-chain: {:?}, indexer: {:?}",
                    leaves_hash_chain,
                    indexer_hashchain
                );
                yield Err(ForesterUtilsError::Indexer("Hashchain mismatch between indexer and on-chain state".into()))
            }

            let circuit_inputs = match get_batch_append_inputs_v2::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                old_root,
                adjusted_start_index,
                leaves.clone(),
                *leaves_hash_chain,
                old_leaves.clone(),
                merkle_proofs.clone(),
                zkp_batch_size as u32,
                new_root,
            ) {
                Ok(inputs) => {
                    debug!("📍 APPEND batch {} circuit inputs: old_root={:?}[..4] new_root={:?}[..4] start_index={} leaves={}",
                        batch_idx, &old_root[..4], &new_root[..4], adjusted_start_index, leaves.len());
                    inputs
                },
                Err(e) => {
                    yield Err(ForesterUtilsError::Prover(format!("Failed to get circuit inputs: {}", e)));
                    return;
                }
            };

            expected_indexer_root = new_root;

            {
                let mut cache = staging_tree_cache.lock().await;
                *cache = staging_tree.clone();
            }

            let client = Arc::clone(&append_proof_client);
            let (proof, new_root) = match client.generate_batch_append_proof(circuit_inputs).await {
                Ok(result) => {
                    result
                },
                Err(e) => {
                    yield Err(ForesterUtilsError::Prover(e.to_string()));
                    return;
                }
            };

            let instruction_data = InstructionDataBatchAppendInputs {
                new_root,
                compressed_proof: CompressedProof {
                    a: proof.a,
                    b: proof.b,
                    c: proof.c,
                },
            };

            debug!("Generated APPEND instruction data for batch {}", batch_idx);
            proofs_buffer.push(instruction_data);

            if proofs_buffer.len() >= MAX_PROOFS_PER_TX {
                yield Ok(BatchInstruction::Append(proofs_buffer.clone()));
                proofs_buffer.clear();
                prefetched_input_queue = None;
            }
        }

        if !proofs_buffer.is_empty() {
            yield Ok(BatchInstruction::Append(proofs_buffer));
            prefetched_input_queue = None;
        }

        let mut proofs_buffer = Vec::new();

        for (batch_idx, leaves_hash_chain) in nullify_hash_chains.iter().enumerate() {
            if !proofs_buffer.is_empty() && batch_idx > 0 {
                yield Ok(BatchInstruction::Nullify(proofs_buffer.clone()));
                proofs_buffer.clear();
            }

            let batch_data = if batch_idx == 0 && prefetched_input_queue.is_some() {
                prefetched_input_queue.take().unwrap()
            } else {
                let queue_elements_result = {
                    let mut connection = rpc_pool.get_connection().await?;
                    let indexer = connection.indexer_mut()?;
                    use light_client::indexer::QueueElementsV2Options;
                    let options = QueueElementsV2Options::default()
                        .with_input_queue(next_nullify_queue_index, Some(zkp_batch_size))
                        .with_input_queue_batch_size(Some(zkp_batch_size));
                    indexer
                        .get_queue_elements_v2(
                            merkle_tree_pubkey.to_bytes(),
                            options,
                            None,
                        )
                        .await
                };

                match queue_elements_result {
                    Ok(res) => {
                        let input_queue = res.value.input_queue.ok_or_else(|| {
                            ForesterUtilsError::Indexer("No input queue data in V2 response".into())
                        })?;

                        if input_queue.leaf_indices.len() != zkp_batch_size as usize {
                            warn!(
                                "Got {} elements but expected {}, stopping NULLIFY phase",
                                input_queue.leaf_indices.len(), zkp_batch_size
                            );
                            break;
                        }

                        input_queue
                    },
                    Err(e) => {
                        yield Err(ForesterUtilsError::Indexer(format!("Failed to get queue elements for NULLIFY batch {}: {}", batch_idx, e)));
                        return;
                    }
                }
            };

            if batch_data.initial_root != expected_indexer_root {
                debug!(
                    "Root mismatch for NULLIFY batch {}: indexer root {:?} != expected root {:?}",
                    batch_idx, batch_data.initial_root, expected_indexer_root
                );
                yield Err(ForesterUtilsError::Indexer("Root mismatch between indexer and expected state".into()));
                return;
            }

            next_nullify_queue_index = Some(batch_data.first_queue_index + zkp_batch_size as u64);
            debug!("Next NULLIFY batch will start at queue index: {}", batch_data.first_queue_index + zkp_batch_size as u64);

            if staging_tree.is_none() {
                match StagingTree::from_v2_input_queue(
                    &batch_data.leaf_indices,
                    &batch_data.current_leaves,
                    &batch_data.nodes,
                    &batch_data.node_hashes,
                ) {
                    Ok(tree) => {
                        staging_tree = Some(tree);
                    },
                    Err(e) => {
                        yield Err(ForesterUtilsError::Prover(format!("Failed to initialize staging tree: {}", e)));
                        return;
                    }
                }
            }

            let staging = staging_tree.as_mut().unwrap();

            let leaves = batch_data.account_hashes.clone();
            let tx_hashes = batch_data.tx_hashes.clone();

            let path_indices: Vec<u32> = batch_data.leaf_indices.iter().map(|&idx| idx as u32).collect();

            let mut nullifiers = Vec::with_capacity(leaves.len());
            for i in 0..leaves.len() {
                let mut leaf_index_bytes = [0u8; 32];
                leaf_index_bytes[24..].copy_from_slice(batch_data.leaf_indices[i].to_be_bytes().as_slice());

                let nullifier = match Poseidon::hashv(&[&leaves[i], &leaf_index_bytes, &tx_hashes[i]]) {
                    Ok(hash) => hash,
                    Err(e) => {
                        yield Err(ForesterUtilsError::StagingTree(format!("Failed to compute nullifier for index {}: {}", i, e)));
                        return;
                    }
                };
                nullifiers.push(nullifier);
            }

            let (old_leaves, merkle_proofs, old_root, new_root) = match staging.process_batch_updates(
                &batch_data.leaf_indices,
                &nullifiers,
                "NULLIFY",
                batch_idx,
            ) {
                Ok(result) => result,
                Err(e) => {
                    yield Err(ForesterUtilsError::StagingTree(format!("Failed to process NULLIFY batch {}: {}", batch_idx, e)));
                    return;
                }
            };

            let circuit_inputs = match get_batch_update_inputs_v2::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                old_root,
                tx_hashes.clone(),
                nullifiers.clone(),
                *leaves_hash_chain,
                old_leaves.clone(),
                merkle_proofs.clone(),
                path_indices.clone(),
                zkp_batch_size as u32,
                new_root,
            ) {
                Ok(inputs) => {
                    debug!("📍 NULLIFY batch {} circuit inputs: old_root={:?}[..4] new_root={:?}[..4] leaves={}",
                        batch_idx, &old_root[..4], &new_root[..4], nullifiers.len());
                    inputs
                },
                Err(e) => {
                    yield Err(ForesterUtilsError::Prover(format!("Failed to get batch update inputs: {}", e)));
                    return;
                }
            };

            expected_indexer_root = new_root;

            {
                let mut cache = staging_tree_cache.lock().await;
                *cache = staging_tree.clone();
            }

            let client = Arc::clone(&nullify_proof_client);
            let (proof, new_root) = match client.generate_batch_update_proof(circuit_inputs).await {
                Ok(result) => {
                    result
                },
                Err(e) => {
                    yield Err(ForesterUtilsError::Prover(e.to_string()));
                    return;
                }
            };

            let instruction_data = InstructionDataBatchNullifyInputs {
                new_root,
                compressed_proof: CompressedProof {
                    a: proof.a,
                    b: proof.b,
                    c: proof.c,
                },
            };

            proofs_buffer.push(instruction_data);

            if proofs_buffer.len() >= MAX_PROOFS_PER_TX {
                yield Ok(BatchInstruction::Nullify(proofs_buffer.clone()));
                proofs_buffer.clear();
            }
        }

        if !proofs_buffer.is_empty() {
            yield Ok(BatchInstruction::Nullify(proofs_buffer));
        }
    };

    Ok((Box::pin(stream), zkp_batch_size))
}
