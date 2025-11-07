use std::{pin::Pin, sync::Arc, time::Duration};

use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use async_stream::stream;
use futures::stream::Stream;
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT, merkle_tree::InstructionDataBatchAppendInputs,
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_hasher::bigint::bigint_to_be_bytes_array;
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::batch_append::{get_batch_append_inputs, BatchAppendsCircuitInputs},
};
use light_sparse_merkle_tree::changelog::ChangelogEntry;
use tracing::{debug, error, trace, warn};

use crate::{
    error::ForesterUtilsError, rpc_pool::SolanaRpcPool, utils::wait_for_indexer,
    ParsedMerkleTreeData, ParsedQueueData,
};

const MAX_PROOFS_PER_TX: usize = 3;

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
) -> Result<
    (
        Pin<
            Box<
                dyn Stream<Item = Result<Vec<InstructionDataBatchAppendInputs>, ForesterUtilsError>>
                    + Send
                    + 'a,
            >,
        >,
        u16,
    ),
    ForesterUtilsError,
> {
    trace!("Initializing append batch instruction stream with parsed data");
    let (merkle_tree_next_index, mut current_root, _) = (
        merkle_tree_data.next_index,
        merkle_tree_data.current_root,
        merkle_tree_data.root_history,
    );
    let (zkp_batch_size, leaves_hash_chains) = (
        output_queue_data.zkp_batch_size,
        output_queue_data.leaves_hash_chains,
    );

    if leaves_hash_chains.is_empty() {
        trace!("No hash chains to process, returning empty stream.");
        return Ok((Box::pin(futures::stream::empty()), zkp_batch_size));
    }
    let rpc = rpc_pool.get_connection().await?;
    wait_for_indexer(&*rpc).await?;
    drop(rpc);

    let stream = stream! {
        let mut next_queue_index: Option<u64> = None;

        let mut all_changelogs: Vec<ChangelogEntry<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>> = Vec::new();

        let proof_client = Arc::new(ProofClient::with_config(prover_url.clone(), polling_interval, max_wait_time, prover_api_key.clone()));

        let mut expected_indexer_root = current_root;
        let mut proofs_buffer = Vec::new();

        for (batch_idx, leaves_hash_chain) in leaves_hash_chains.iter().enumerate() {
            if !proofs_buffer.is_empty() && batch_idx > 0 {
                debug!("Have {} accumulated proofs before fetching batch {}", proofs_buffer.len(), batch_idx);
                yield Ok(proofs_buffer.clone());
                proofs_buffer.clear();
                debug!("Waiting for transaction to land and indexer to sync...");
                let rpc = rpc_pool.get_connection().await?;
                match wait_for_indexer(&*rpc).await {
                    Ok(_) => {
                        expected_indexer_root = current_root;
                        debug!("Transaction landed, updated expected root for batch {}", batch_idx);
                    }
                    Err(e) => {
                        debug!("Could not sync with indexer, likely phase ended: {}", e);
                        return;
                    }
                }
                drop(rpc);
            }

            let queue_elements_result = {
                let mut connection = rpc_pool.get_connection().await?;
                let indexer = connection.indexer_mut()?;
                indexer
                    .get_queue_elements(
                        merkle_tree_pubkey.to_bytes(),
                        next_queue_index,
                        Some(zkp_batch_size),
                        None,
                        None,
                        None,
                    )
                    .await
            };

            let (batch_elements, batch_first_queue_idx) = match queue_elements_result {
                Ok(res) => {
                    let items = res.value.output_queue_elements.unwrap_or_default();
                    let first_idx = res.value.output_queue_index;
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
                    error!(
                        "Root mismatch! Indexer root: {:?}, Expected root: {:?}, indexer seq: {}, first_element.leaf_index: {}",
                        first_element.root,
                        expected_indexer_root,
                        first_element.root_seq,
                        first_element.leaf_index
                    );
                    yield Err(ForesterUtilsError::Indexer("Root mismatch between indexer and expected state".into()));
                    return;
                }
            }

            if let Some(first_idx) = batch_first_queue_idx {
                next_queue_index = Some(first_idx + zkp_batch_size as u64);
                debug!("Next batch will start at queue index: {:?}", next_queue_index);
            }

            let old_leaves: Vec<[u8; 32]> = batch_elements.iter().map(|x| x.leaf).collect();
            let leaves: Vec<[u8; 32]> = batch_elements.iter().map(|x| x.account_hash).collect();
            let merkle_proofs: Vec<Vec<[u8; 32]>> = batch_elements.iter().map(|x| x.proof.clone()).collect();
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

            let (circuit_inputs, batch_changelogs) = match get_batch_append_inputs::<32>(
                current_root, adjusted_start_index, leaves, *leaves_hash_chain, old_leaves, merkle_proofs, zkp_batch_size as u32, &all_changelogs,
            ) {
                Ok(inputs) => {
                    debug!("Batch append circuit inputs created successfully ({}, {})", inputs.0.start_index, inputs.0.batch_size);
                    inputs
                },
                Err(e) => {
                    yield Err(ForesterUtilsError::Prover(format!("Failed to get circuit inputs: {}", e)));
                    return;
                }
            };

            current_root = bigint_to_be_bytes_array::<32>(&circuit_inputs.new_root.to_biguint().unwrap()).unwrap();
            all_changelogs.extend(batch_changelogs);

            let client = Arc::clone(&proof_client);
            match generate_zkp_proof(circuit_inputs, client).await {
                Ok(proof) => {
                    debug!("Generated proof for batch {}", batch_idx);
                    proofs_buffer.push(proof);

                    if proofs_buffer.len() >= MAX_PROOFS_PER_TX {
                        debug!("Buffer full with {} proofs, yielding for transaction", proofs_buffer.len());
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
