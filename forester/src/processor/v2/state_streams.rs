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
use light_hasher::Poseidon;
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

use super::tree_cache;
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
    let tree_cache = tree_cache::get_tree_cache().await;
    
    let stream = stream! {
        let total_elements = zkp_batch_size as usize * num_batches_to_process;
        let mut current_root = merkle_tree_data.current_root;
        let offset = merkle_tree_data.num_inserted_zkps * zkp_batch_size as u64;

        trace!("Starting nullify stream - total_elements: {}, offset: {}", total_elements, offset);
        
        // Get tree snapshot from cache or create from on-chain data
        let tree_snapshot = match tree_cache.get(&merkle_tree_pubkey).await {
            Some(snapshot) if snapshot.root == current_root => {
                info!("Using cached tree snapshot for nullify");
                snapshot
            }
            _ => {
                info!("Tree cache miss or stale, fetching subtrees from indexer");
                // Fetch subtrees from indexer
                let mut rpc = match rpc_pool.get_connection().await {
                    Ok(rpc) => rpc,
                    Err(e) => {
                        yield Err(anyhow!("RPC error: {}", e));
                        return;
                    }
                };
                
                let indexer = match rpc.indexer_mut() {
                    Ok(indexer) => indexer,
                    Err(e) => {
                        yield Err(anyhow!("Indexer error: {}", e));
                        return;
                    }
                };
                
                let subtrees_response = match indexer.get_subtrees(merkle_tree_pubkey.to_bytes(), None).await {
                    Ok(res) => res,
                    Err(e) => {
                        yield Err(anyhow!("Failed to get subtrees: {}", e));
                        return;
                    }
                };
                
                let subtrees = subtrees_response.value.items;
                
                // Update cache with fresh data
                if let Err(e) = tree_cache.update_from_data(
                    merkle_tree_pubkey,
                    subtrees.clone(),
                    merkle_tree_data.next_index as usize,
                    current_root,
                    DEFAULT_BATCH_STATE_TREE_HEIGHT as usize,
                ).await {
                    yield Err(anyhow!("Failed to update tree cache: {}", e));
                    return;
                }
                
                match tree_cache.get(&merkle_tree_pubkey).await {
                    Some(snapshot) => snapshot,
                    None => {
                        yield Err(anyhow!("Failed to get tree snapshot after update"));
                        return;
                    }
                }
            }
        };
        
        // Create local tree from snapshot to track changes
        let mut local_tree = match tree_snapshot.to_tree::<Poseidon, { DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>() {
            Ok(tree) => tree,
            Err(e) => {
                yield Err(anyhow!("Failed to create tree from snapshot: {}", e));
                return;
            }
        };

        let all_queue_elements = {
            let mut connection = match rpc_pool.get_connection().await {
                Ok(conn) => conn,
                Err(e) => {
                    yield Err(anyhow::Error::from(ForesterUtilsError::Indexer(format!("RPC error: {}", e))));
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

        let mut all_changelogs = Vec::new();
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
                &all_changelogs,
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
                        current_root = proof_data.new_root;
                        
                        // Update local tree to reflect the nullification
                        // Note: For nullify, the tree structure doesn't change, only the root
                        
                        proof_buffer.push(proof_data);
                        
                        if proof_buffer.len() >= yield_batch_size || (batch_offset == num_batches_to_process - 1 && pending_count == 0) {
                            // Update tree cache with new state after processing batch
                            if let Err(e) = tree_cache.update_from_data(
                                merkle_tree_pubkey,
                                local_tree.get_subtrees().to_vec(),
                                local_tree.get_next_index(),
                                current_root,
                                DEFAULT_BATCH_STATE_TREE_HEIGHT as usize,
                            ).await {
                                yield Err(anyhow!("Failed to update tree cache: {}", e));
                                return;
                            }
                            
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
    let tree_cache = tree_cache::get_tree_cache().await;
    
    let stream = stream! {
        let total_elements = zkp_batch_size as usize * num_batches_to_process;
        let mut current_root = merkle_tree_data.current_root;
        let offset = merkle_tree_data.next_index;
        let mut current_next_index = merkle_tree_data.next_index as u32;

        trace!("Starting append stream - total_elements: {}, offset: {}", total_elements, offset);
        
        // Get tree snapshot from cache or create from on-chain data
        let tree_snapshot = match tree_cache.get(&merkle_tree_pubkey).await {
            Some(snapshot) if snapshot.root == current_root && snapshot.next_index == current_next_index as usize => {
                info!("Using cached tree snapshot for append");
                snapshot
            }
            _ => {
                info!("Tree cache miss or stale, fetching subtrees from indexer");
                // Fetch subtrees from indexer
                let mut rpc = match rpc_pool.get_connection().await {
                    Ok(rpc) => rpc,
                    Err(e) => {
                        yield Err(anyhow!("RPC error: {}", e));
                        return;
                    }
                };
                
                let indexer = match rpc.indexer_mut() {
                    Ok(indexer) => indexer,
                    Err(e) => {
                        yield Err(anyhow!("Indexer error: {}", e));
                        return;
                    }
                };
                
                let subtrees_response = match indexer.get_subtrees(merkle_tree_pubkey.to_bytes(), None).await {
                    Ok(res) => res,
                    Err(e) => {
                        yield Err(anyhow!("Failed to get subtrees: {}", e));
                        return;
                    }
                };
                
                let subtrees = subtrees_response.value.items;
                
                // Update cache with fresh data
                if let Err(e) = tree_cache.update_from_data(
                    merkle_tree_pubkey,
                    subtrees.clone(),
                    current_next_index as usize,
                    current_root,
                    DEFAULT_BATCH_STATE_TREE_HEIGHT as usize,
                ).await {
                    yield Err(anyhow!("Failed to update tree cache: {}", e));
                    return;
                }
                
                match tree_cache.get(&merkle_tree_pubkey).await {
                    Some(snapshot) => snapshot,
                    None => {
                        yield Err(anyhow!("Failed to get tree snapshot after update"));
                        return;
                    }
                }
            }
        };
        
        // Create local tree from snapshot to track changes
        let mut local_tree = match tree_snapshot.to_tree::<Poseidon, { DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>() {
            Ok(tree) => tree,
            Err(e) => {
                yield Err(anyhow!("Failed to create tree from snapshot: {}", e));
                return;
            }
        };

        let queue_elements = {
            let mut connection = match rpc_pool.get_connection().await {
                Ok(conn) => conn,
                Err(e) => {
                    yield Err(anyhow::Error::from(ForesterUtilsError::Indexer(format!("RPC error: {}", e))));
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

        let mut all_changelogs: Vec<ChangelogEntry<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>> = Vec::new();
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
                &all_changelogs,
            ) {
                Ok(inputs) => inputs,
                Err(e) => {
                    yield Err(anyhow!("Failed to get batch append inputs: {}", e));
                    return;
                }
            };

            all_changelogs.extend(batch_changelogs);
            
            // Update local tree with the new leaves
            for leaf in &leaves {
                local_tree.append(*leaf);
            }

            let proof_client = proof_client.clone();
            let future = Box::pin(generate_append_zkp_proof(circuit_inputs, proof_client));
            futures_ordered.push_back(future);
            pending_count += 1;

            while pending_count >= yield_batch_size || (batch_idx == num_batches_to_process - 1 && pending_count > 0) {
                match futures_ordered.next().await {
                    Some(Ok(proof_data)) => {
                        pending_count -= 1;
                        current_root = proof_data.new_root;
                        
                        // Local tree is updated as we process batches
                        // The tree structure changes are tracked in the tree cache
                        current_next_index += zkp_batch_size as u32;
                        
                        proof_buffer.push(proof_data);
                        
                        if proof_buffer.len() >= yield_batch_size || (batch_idx == num_batches_to_process - 1 && pending_count == 0) {
                            // Update tree cache with new state after processing batch
                            if let Err(e) = tree_cache.update_from_data(
                                merkle_tree_pubkey,
                                local_tree.get_subtrees().to_vec(),
                                local_tree.get_next_index(),
                                current_root,
                                DEFAULT_BATCH_STATE_TREE_HEIGHT as usize,
                            ).await {
                                yield Err(anyhow!("Failed to update tree cache: {}", e));
                                return;
                            }
                            
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

        if !proof_buffer.is_empty() {
            yield Ok(proof_buffer);
        }
    };

    Ok((Box::pin(stream), zkp_batch_size))
}