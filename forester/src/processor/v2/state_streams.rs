use std::{pin::Pin, sync::Arc, time::Duration};
use futures::future::join_all;

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

const MAX_PROOF_SIZE : usize = 3;
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
    prover_api_key: Option<String>,
    polling_interval: Duration,
    max_wait_time: Duration,
    merkle_tree_data: ParsedMerkleTreeData,
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
                Ok(res) => res.value.0,
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
        let proof_client = Arc::new(ProofClient::with_config(
            prover_url.clone(),
            polling_interval,
            max_wait_time,
            prover_api_key,
        ));
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

            while pending_count >= MAX_PROOF_SIZE || (batch_offset == num_batches_to_process - 1 && pending_count > 0) {
                match futures_ordered.next().await {
                    Some(Ok(proof_data)) => {
                        pending_count -= 1;
                        proof_buffer.push(proof_data);
                        
                        if proof_buffer.len() >= MAX_PROOF_SIZE || (batch_offset == num_batches_to_process - 1 && pending_count == 0) {
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
    };

    Ok((Box::pin(stream), zkp_batch_size))
}

/// Prepare proofs for both nullify and append with sequential changelog calculation
/// but parallel proof generation for maximum performance
#[allow(clippy::too_many_arguments)]
pub async fn prepare_proofs_with_sequential_changelogs<R: Rpc>(
    rpc_pool: Arc<SolanaRpcPool<R>>,
    merkle_tree_pubkey: Pubkey,
    nullify_prover_url: String,
    append_prover_url: String,
    prover_api_key: Option<String>,
    polling_interval: Duration,
    max_wait_time: Duration,
    merkle_tree_data: ParsedMerkleTreeData,
    output_queue_data: ParsedQueueData,
) -> AnyhowResult<(Vec<InstructionDataBatchNullifyInputs>, Vec<InstructionDataBatchAppendInputs>)> {
    info!("Preparing proofs with optimized parallel generation");
    
    let nullify_zkp_batch_size = merkle_tree_data.zkp_batch_size;
    let append_zkp_batch_size = output_queue_data.zkp_batch_size;
    let nullify_leaves_hash_chains = merkle_tree_data.leaves_hash_chains.clone();
    let append_leaves_hash_chains = output_queue_data.leaves_hash_chains.clone();
    
    // Early return if nothing to process
    if nullify_leaves_hash_chains.is_empty() && append_leaves_hash_chains.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    
    // Step 1: Fetch queue elements in parallel for both operations
    let (nullify_elements, append_elements) = {
        let nullify_future = async {
            if nullify_leaves_hash_chains.is_empty() {
                return Ok(Vec::new());
            }
            let mut connection = rpc_pool.get_connection().await?;
            let indexer = connection.indexer_mut()?;
            let total_elements = nullify_zkp_batch_size as usize * nullify_leaves_hash_chains.len();
            let offset = merkle_tree_data.num_inserted_zkps * nullify_zkp_batch_size as u64;
            
            let res = indexer.get_queue_elements(
                merkle_tree_pubkey.to_bytes(),
                QueueType::InputStateV2,
                total_elements as u16,
                Some(offset),
                None,
            ).await?;
            Ok::<_, anyhow::Error>(res.value.0)
        };
        
        let append_future = async {
            if append_leaves_hash_chains.is_empty() {
                return Ok(Vec::new());
            }
            let mut connection = rpc_pool.get_connection().await?;
            let indexer = connection.indexer_mut()?;
            let total_elements = append_zkp_batch_size as usize * append_leaves_hash_chains.len();
            let offset = merkle_tree_data.next_index;
            
            let res = indexer.get_queue_elements(
                merkle_tree_pubkey.to_bytes(),
                QueueType::OutputStateV2,
                total_elements as u16,
                Some(offset),
                None,
            ).await?;
            Ok::<_, anyhow::Error>(res.value.0)
        };
        
        futures::join!(nullify_future, append_future)
    };
    
    let nullify_queue_elements = nullify_elements?;
    let append_queue_elements = append_elements?;
    
    // Step 2: Get cached changelogs
    let changelog_cache = changelog_cache::get_changelog_cache().await;
    let previous_changelogs = changelog_cache.get_changelogs(&merkle_tree_pubkey).await;
    info!("Starting with {} cached changelogs", previous_changelogs.len());
    
    // Step 3: Calculate nullify changelogs first (sequential)
    let mut all_changelogs: Vec<ChangelogEntry<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>> = previous_changelogs.clone();
    let mut nullify_circuit_inputs = Vec::new();
    let mut current_root = merkle_tree_data.current_root;
    
    for (batch_offset, leaves_hash_chain) in nullify_leaves_hash_chains.iter().enumerate() {
        let start_idx = batch_offset * nullify_zkp_batch_size as usize;
        let end_idx = start_idx + nullify_zkp_batch_size as usize;
        let batch_elements = &nullify_queue_elements[start_idx..end_idx];
        
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
            tx_hashes.push(leaf_info.tx_hash.ok_or_else(|| {
                anyhow!("Missing tx_hash for leaf index {}", leaf_info.leaf_index)
            })?);
        }
        
        let (circuit_inputs, batch_changelog) = get_batch_update_inputs::<
            { DEFAULT_BATCH_STATE_TREE_HEIGHT as usize },
        >(
            current_root,  // Use the current root, which gets updated after each batch
            tx_hashes,
            leaves,
            *leaves_hash_chain,
            old_leaves,
            merkle_proofs,
            path_indices,
            nullify_zkp_batch_size as u32,
            &all_changelogs,  // Use accumulated changelogs
        )?;
        
        // Update current_root to the new root from this batch for the next iteration
        // The new root is in the circuit_inputs, convert from BigInt back to bytes
        let new_root_bytes = circuit_inputs.new_root.to_bytes_be().1;
        if new_root_bytes.len() == 32 {
            current_root.copy_from_slice(&new_root_bytes);
            debug!("Updated root after nullify batch {}: {:?}", batch_offset, current_root);
        } else {
            // Pad or truncate to 32 bytes if necessary
            current_root = [0u8; 32];
            let offset = 32usize.saturating_sub(new_root_bytes.len());
            current_root[offset..].copy_from_slice(&new_root_bytes[..new_root_bytes.len().min(32)]);
            debug!("Updated root after nullify batch {} (padded): {:?}", batch_offset, current_root);
        }
        
        all_changelogs.extend(batch_changelog);
        nullify_circuit_inputs.push(circuit_inputs);
    }
    
    info!("Calculated {} nullify changelogs", all_changelogs.len() - previous_changelogs.len());
    
    // Step 4: Calculate append inputs with nullify's changelogs
    // Continue using the current_root from where nullify left off
    let mut append_circuit_inputs = Vec::new();
    
    for (batch_idx, leaves_hash_chain) in append_leaves_hash_chains.iter().enumerate() {
        let start_idx = batch_idx * append_zkp_batch_size as usize;
        let end_idx = start_idx + append_zkp_batch_size as usize;
        let batch_elements = &append_queue_elements[start_idx..end_idx];
        
        let new_leaves: Vec<[u8; 32]> = batch_elements.iter().map(|x| x.account_hash).collect();
        let merkle_proofs: Vec<Vec<[u8; 32]>> = batch_elements.iter().map(|x| x.proof.clone()).collect();
        let adjusted_start_index = merkle_tree_data.next_index as u32 + (batch_idx * append_zkp_batch_size as usize) as u32;
        let old_leaves: Vec<[u8; 32]> = batch_elements.iter().map(|x| x.leaf).collect();
        
        let (circuit_inputs, batch_changelog) = get_batch_append_inputs::<
            { DEFAULT_BATCH_STATE_TREE_HEIGHT as usize },
        >(
            current_root,  // Use the current root, which was updated by nullify operations
            adjusted_start_index,
            new_leaves,
            *leaves_hash_chain,
            old_leaves,
            merkle_proofs,
            append_zkp_batch_size as u32,
            &all_changelogs,  // Use changelogs including nullify's
        )?;
        
        // Update current_root for the next append batch
        // The new root is in the circuit_inputs, convert from BigInt back to bytes
        let new_root_bytes = circuit_inputs.new_root.to_bytes_be().1;
        if new_root_bytes.len() == 32 {
            current_root.copy_from_slice(&new_root_bytes);
            debug!("Updated root after append batch {}: {:?}", batch_idx, current_root);
        } else {
            // Pad or truncate to 32 bytes if necessary
            current_root = [0u8; 32];
            let offset = 32usize.saturating_sub(new_root_bytes.len());
            current_root[offset..].copy_from_slice(&new_root_bytes[..new_root_bytes.len().min(32)]);
            debug!("Updated root after append batch {} (padded): {:?}", batch_idx, current_root);
        }
        
        all_changelogs.extend(batch_changelog);
        append_circuit_inputs.push(circuit_inputs);
    }
    
    info!("Calculated {} append changelogs", all_changelogs.len() - previous_changelogs.len() - nullify_circuit_inputs.len());
    
    // Step 5: Generate all proofs in parallel (this is the expensive part)
    let nullify_proof_client = Arc::new(ProofClient::with_config(
        nullify_prover_url,
        polling_interval,
        max_wait_time,
        prover_api_key.clone(),
    ));
    
    let append_proof_client = Arc::new(ProofClient::with_config(
        append_prover_url,
        polling_interval,
        max_wait_time,
        prover_api_key,
    ));
    
    // Generate nullify proofs
    let mut nullify_futures = Vec::new();
    for inputs in nullify_circuit_inputs {
        let client = nullify_proof_client.clone();
        nullify_futures.push(generate_nullify_zkp_proof(inputs, client));
    }
    
    // Generate append proofs
    let mut append_futures = Vec::new();
    for inputs in append_circuit_inputs {
        let client = append_proof_client.clone();
        append_futures.push(generate_append_zkp_proof(inputs, client));
    }
    
    info!("Generating {} proofs in parallel ({} nullify, {} append)", 
          nullify_futures.len() + append_futures.len(), 
          nullify_futures.len(), 
          append_futures.len());
    
    // Execute all proof generation in parallel
    let (nullify_results, append_results) = futures::join!(
        join_all(nullify_futures),
        join_all(append_futures)
    );
    
    // Collect nullify proofs
    let mut nullify_proofs = Vec::new();
    for result in nullify_results {
        match result {
            Ok(proof) => nullify_proofs.push(proof),
            Err(e) => return Err(e.into()),
        }
    }
    
    // Collect append proofs
    let mut append_proofs = Vec::new();
    for result in append_results {
        match result {
            Ok(proof) => append_proofs.push(proof),
            Err(e) => return Err(e.into()),
        }
    }
    
    // Step 6: Cache the new changelogs for future use
    let new_changelogs = all_changelogs.into_iter().skip(previous_changelogs.len()).collect::<Vec<_>>();
    if !new_changelogs.is_empty() {
        changelog_cache.append_changelogs(merkle_tree_pubkey, new_changelogs.clone()).await?;
        info!("Cached {} new changelogs for future operations", new_changelogs.len());
    }
    
    info!("Generated {} nullify and {} append proofs", nullify_proofs.len(), append_proofs.len());
    Ok((nullify_proofs, append_proofs))
}

#[allow(clippy::too_many_arguments)]
pub async fn get_append_instruction_stream<'a, R: Rpc>(
    rpc_pool: Arc<SolanaRpcPool<R>>,
    merkle_tree_pubkey: Pubkey,
    prover_url: String,
    prover_api_key: Option<String>,
    polling_interval: Duration,
    max_wait_time: Duration,
    merkle_tree_data: ParsedMerkleTreeData,
    output_queue_data: ParsedQueueData,
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
                Ok(res) => res.value.0,
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
        let proof_client = Arc::new(ProofClient::with_config(
            prover_url.clone(),
            polling_interval,
            max_wait_time,
            prover_api_key,
        ));
        let mut futures_ordered = FuturesOrdered::new();
        let mut pending_count = 0;
        let mut proof_buffer = Vec::new();

        for (batch_idx, leaves_hash_chain) in leaves_hash_chains.iter().enumerate() {
            let start_idx = batch_idx * zkp_batch_size as usize;
            let end_idx = start_idx + zkp_batch_size as usize;
            let batch_elements = &queue_elements[start_idx..end_idx];

            let new_leaves: Vec<[u8; 32]> = batch_elements.iter().map(|x| x.account_hash).collect();
            let merkle_proofs: Vec<Vec<[u8; 32]>> = batch_elements.iter().map(|x| x.proof.clone()).collect();
            let adjusted_start_index = offset as u32 + (batch_idx * zkp_batch_size as usize) as u32;

            // The queue elements contain the new leaves to append
            // For append, old_leaves at these positions are typically zeros (empty slots)
            let old_leaves: Vec<[u8; 32]> = batch_elements.iter().map(|x| x.leaf).collect();
            
            // Pass previous changelogs to get_batch_append_inputs
            let (circuit_inputs, batch_changelog) = match get_batch_append_inputs::<
                { DEFAULT_BATCH_STATE_TREE_HEIGHT as usize },
            >(
                current_root,
                adjusted_start_index,
                new_leaves.clone(),
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

            all_changelogs.extend(batch_changelog);

            let proof_client = proof_client.clone();
            let future = Box::pin(generate_append_zkp_proof(circuit_inputs, proof_client));
            futures_ordered.push_back(future);
            pending_count += 1;

            while pending_count >= MAX_PROOF_SIZE || (batch_idx == num_batches_to_process - 1 && pending_count > 0) {
                match futures_ordered.next().await {
                    Some(Ok(proof_data)) => {
                        pending_count -= 1;
                        proof_buffer.push(proof_data);
                        
                        if proof_buffer.len() >= MAX_PROOF_SIZE || (batch_idx == num_batches_to_process - 1 && pending_count == 0) {
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
    };

    Ok((Box::pin(stream), zkp_batch_size))
}