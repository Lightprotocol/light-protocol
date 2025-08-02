use std::{pin::Pin, sync::Arc, time::Duration};

use async_stream::stream;
use futures::{
    stream::{FuturesOrdered, Stream},
    StreamExt,
};
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT,
    merkle_tree::{InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs},
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_merkle_tree_metadata::QueueType;
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::{
        batch_append::{get_batch_append_inputs, BatchAppendsCircuitInputs},
        batch_update::{get_batch_update_inputs, BatchUpdateCircuitInputs},
    },
};
use light_sparse_merkle_tree::changelog::ChangelogEntry;
use solana_sdk::pubkey::Pubkey;
use tracing::{debug, trace, info};

use super::changelog_cache;
use forester_utils::{
    error::ForesterUtilsError, 
    rpc_pool::SolanaRpcPool, 
    ParsedMerkleTreeData, 
    ParsedQueueData,
};
use anyhow::{anyhow, Result as AnyhowResult};

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

async fn generate_append_zkp_proof(
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

#[allow(clippy::too_many_arguments)]
pub async fn get_nullify_instruction_stream<'a, R: Rpc>(
    rpc_pool: Arc<SolanaRpcPool<R>>,
    merkle_tree_pubkey: Pubkey,
    prover_url: String,
    polling_interval: Duration,
    max_wait_time: Duration,
    merkle_tree_data: ParsedMerkleTreeData,
    yield_batch_size: usize,
) -> AnyhowResult<
    (
        Pin<
            Box<
                dyn Stream<
                        Item = Result<Vec<InstructionDataBatchNullifyInputs>, anyhow::Error>,
                    > + Send
                    + 'a,
            >,
        >,
        u16,
    ),
> {
    let zkp_batch_size = merkle_tree_data.zkp_batch_size;
    let leaves_hash_chains = merkle_tree_data.leaves_hash_chains.clone();
    
    if leaves_hash_chains.is_empty() {
        debug!("No hash chains to process for nullification");
        return Ok((Box::pin(futures::stream::empty()), zkp_batch_size));
    }

    let num_batches_to_process = leaves_hash_chains.len();
    let changelog_cache = changelog_cache::get_changelog_cache().await;
    
    let stream = stream! {
        let total_elements = zkp_batch_size as usize * num_batches_to_process;
        let current_root = merkle_tree_data.current_root;
        let offset = merkle_tree_data.num_inserted_zkps * zkp_batch_size as u64;

        trace!("Starting nullify stream - total_elements: {}, offset: {}", total_elements, offset);
        
        // Get accumulated changelogs from cache
        let previous_changelogs = changelog_cache.get_changelogs(&merkle_tree_pubkey).await;
        info!("Using {} previous changelogs for nullify", previous_changelogs.len());

        // Fetch queue elements with merkle proofs
        let all_queue_elements = {
            let mut connection = match rpc_pool.get_connection().await {
                Ok(conn) => conn,
                Err(e) => {
                    yield Err(anyhow!("RPC error: {}", e));
                    return;
                }
            };
            
            let indexer = match connection.indexer_mut() {
                Ok(indexer) => indexer,
                Err(e) => {
                    yield Err(anyhow!("Indexer error: {}", e));
                    return;
                }
            };
            
            match indexer.get_queue_elements(
                merkle_tree_pubkey.to_bytes(),
                QueueType::InputStateV2,
                total_elements as u16,
                Some(offset),
                None,
            ).await {
                Ok(res) => res.value.items,
                Err(e) => {
                    yield Err(anyhow!("Failed to get queue elements: {}", e));
                    return;
                }
            }
        };

        trace!("Got {} queue elements in total", all_queue_elements.len());
        if all_queue_elements.len() != total_elements {
            yield Err(anyhow!(
                "Expected {} elements, got {}",
                total_elements, all_queue_elements.len()
            ));
            return;
        }

        if let Some(first_element) = all_queue_elements.first() {
            if first_element.root != current_root {
                yield Err(anyhow!("Root mismatch between indexer and on-chain state"));
                return;
            }
        }

        let mut all_changelogs: Vec<ChangelogEntry<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>> = previous_changelogs.clone();
        let proof_client = Arc::new(ProofClient::with_config(prover_url.clone(), polling_interval, max_wait_time));
        let mut futures_ordered = FuturesOrdered::new();
        let mut pending_count = 0;
        let mut proof_buffer = Vec::new();

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
                tx_hashes.push(
                    leaf_info
                        .tx_hash
                        .ok_or(ForesterUtilsError::Indexer(format!(
                            "Missing tx_hash for leaf index {}",
                            leaf_info.leaf_index
                        )))?,
                );
            }

            // Pass previous changelogs to get_batch_update_inputs
            let (circuit_inputs, batch_changelog) = match get_batch_update_inputs::<
                { DEFAULT_BATCH_STATE_TREE_HEIGHT as usize },
            >(
                current_root,
                tx_hashes,
                leaves.clone(),
                *leaves_hash_chain,
                old_leaves,
                merkle_proofs,
                path_indices.clone(),
                zkp_batch_size as u32,
                &previous_changelogs,  // Use cached changelogs
            ) {
                Ok(inputs) => inputs,
                Err(e) => {
                    yield Err(anyhow!("Failed to get batch update inputs: {}", e));
                    return;
                }
            };

            all_changelogs.extend(batch_changelog);

            let proof_client = proof_client.clone();
            let future = Box::pin(generate_nullify_zkp_proof(circuit_inputs, proof_client));
            futures_ordered.push_back(future);
            pending_count += 1;

            while pending_count >= yield_batch_size || (batch_offset == num_batches_to_process - 1 && pending_count > 0) {
                match futures_ordered.next().await {
                    Some(Ok(proof_data)) => {
                        pending_count -= 1;
                        proof_buffer.push(proof_data);
                        
                        if proof_buffer.len() >= yield_batch_size || (batch_offset == num_batches_to_process - 1 && pending_count == 0) {
                            yield Ok(proof_buffer.clone());
                            proof_buffer.clear();
                        }
                    },
                    Some(Err(e)) => {
                        yield Err(e.into());
                        return;
                    },
                    None => break,
                }
            }
        }

        // Store only new changelogs in cache (skip the ones we started with)
        let new_changelogs = all_changelogs.into_iter().skip(previous_changelogs.len()).collect::<Vec<_>>();
        if !new_changelogs.is_empty() {
            if let Err(e) = changelog_cache.append_changelogs(merkle_tree_pubkey, new_changelogs.clone()).await {
                yield Err(anyhow!("Failed to update changelog cache: {}", e));
                return;
            }
            info!("Stored {} new changelogs for nullify", new_changelogs.len());
        }

        if !proof_buffer.is_empty() {
            yield Ok(proof_buffer);
        }
    };

    Ok((Box::pin(stream), zkp_batch_size))
}

#[allow(clippy::too_many_arguments)]
pub async fn get_append_instruction_stream<'a, R: Rpc>(
    rpc_pool: Arc<SolanaRpcPool<R>>,
    merkle_tree_pubkey: Pubkey,
    prover_url: String,
    polling_interval: Duration,
    max_wait_time: Duration,
    merkle_tree_data: ParsedMerkleTreeData,
    output_queue_data: ParsedQueueData,
    yield_batch_size: usize,
) -> AnyhowResult<
    (
        Pin<
            Box<
                dyn Stream<
                        Item = Result<Vec<InstructionDataBatchAppendInputs>, anyhow::Error>,
                    > + Send
                    + 'a,
            >,
        >,
        u16,
    ),
> {
    let zkp_batch_size = output_queue_data.zkp_batch_size;
    let leaves_hash_chains = output_queue_data.leaves_hash_chains.clone();
    
    if leaves_hash_chains.is_empty() {
        debug!("No hash chains to process for append");
        return Ok((Box::pin(futures::stream::empty()), zkp_batch_size));
    }

    let num_batches_to_process = leaves_hash_chains.len();
    let changelog_cache = changelog_cache::get_changelog_cache().await;
    
    let stream = stream! {
        let total_elements = zkp_batch_size as usize * num_batches_to_process;
        let current_root = merkle_tree_data.current_root;
        let offset = merkle_tree_data.next_index;

        trace!("Starting append stream - total_elements: {}, offset: {}", total_elements, offset);
        
        // Get accumulated changelogs from cache
        let previous_changelogs = changelog_cache.get_changelogs(&merkle_tree_pubkey).await;
        info!("Using {} previous changelogs for append", previous_changelogs.len());

        let queue_elements = {
            let mut connection = match rpc_pool.get_connection().await {
                Ok(conn) => conn,
                Err(e) => {
                    yield Err(anyhow!("RPC error: {}", e));
                    return;
                }
            };
            
            let indexer = match connection.indexer_mut() {
                Ok(indexer) => indexer,
                Err(e) => {
                    yield Err(anyhow!("Indexer error: {}", e));
                    return;
                }
            };
            
            match indexer.get_queue_elements(
                merkle_tree_pubkey.to_bytes(),
                QueueType::OutputStateV2,
                total_elements as u16,
                Some(offset),
                None,
            ).await {
                Ok(res) => res.value.items,
                Err(e) => {
                    yield Err(anyhow!("Failed to get queue elements: {}", e));
                    return;
                }
            }
        };

        trace!("Got {} queue elements for append", queue_elements.len());
        if queue_elements.len() != total_elements {
            yield Err(anyhow!(
                "Expected {} elements, got {}",
                total_elements, queue_elements.len()
            ));
            return;
        }

        let mut all_changelogs: Vec<ChangelogEntry<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>> = previous_changelogs.clone();
        let proof_client = Arc::new(ProofClient::with_config(prover_url.clone(), polling_interval, max_wait_time));
        let mut futures_ordered = FuturesOrdered::new();
        let mut pending_count = 0;
        let mut proof_buffer = Vec::new();

        for (batch_idx, leaves_hash_chain) in leaves_hash_chains.iter().enumerate() {
            let start_idx = batch_idx * zkp_batch_size as usize;
            let end_idx = start_idx + zkp_batch_size as usize;
            let batch_elements = &queue_elements[start_idx..end_idx];

            let old_leaves: Vec<[u8; 32]> = batch_elements.iter().map(|x| x.leaf).collect();
            let leaves: Vec<[u8; 32]> = batch_elements.iter().map(|x| x.account_hash).collect();
            let merkle_proofs: Vec<Vec<[u8; 32]>> = batch_elements.iter().map(|x| x.proof.clone()).collect();
            let adjusted_start_index = offset as u32 + (batch_idx * zkp_batch_size as usize) as u32;

            // Pass previous changelogs to get_batch_append_inputs
            let (circuit_inputs, batch_changelogs) = match get_batch_append_inputs::<
                { DEFAULT_BATCH_STATE_TREE_HEIGHT as usize },
            >(
                current_root,
                adjusted_start_index,
                leaves.clone(),
                *leaves_hash_chain,
                old_leaves,
                merkle_proofs,
                zkp_batch_size as u32,
                &previous_changelogs,  // Use cached changelogs
            ) {
                Ok(inputs) => inputs,
                Err(e) => {
                    yield Err(anyhow!("Failed to get batch append inputs: {}", e));
                    return;
                }
            };

            all_changelogs.extend(batch_changelogs);

            let proof_client = proof_client.clone();
            let future = Box::pin(generate_append_zkp_proof(circuit_inputs, proof_client));
            futures_ordered.push_back(future);
            pending_count += 1;

            while pending_count >= yield_batch_size || (batch_idx == num_batches_to_process - 1 && pending_count > 0) {
                match futures_ordered.next().await {
                    Some(Ok(proof_data)) => {
                        pending_count -= 1;
                        proof_buffer.push(proof_data);
                        
                        if proof_buffer.len() >= yield_batch_size || (batch_idx == num_batches_to_process - 1 && pending_count == 0) {
                            yield Ok(proof_buffer.clone());
                            proof_buffer.clear();
                        }
                    },
                    Some(Err(e)) => {
                        yield Err(e.into());
                        return;
                    },
                    None => break,
                }
            }
        }

        // Store only new changelogs in cache (skip the ones we started with)
        let new_changelogs = all_changelogs.into_iter().skip(previous_changelogs.len()).collect::<Vec<_>>();
        if !new_changelogs.is_empty() {
            if let Err(e) = changelog_cache.append_changelogs(merkle_tree_pubkey, new_changelogs.clone()).await {
                yield Err(anyhow!("Failed to update changelog cache: {}", e));
                return;
            }
            info!("Stored {} new changelogs for append", new_changelogs.len());
        }

        if !proof_buffer.is_empty() {
            yield Ok(proof_buffer);
        }
    };

    Ok((Box::pin(stream), zkp_batch_size))
}