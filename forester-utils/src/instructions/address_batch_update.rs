use std::{pin::Pin, sync::Arc, time::Duration};

use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use async_stream::stream;
use futures::{
    stream::{FuturesOrdered, Stream},
    StreamExt,
};
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_ADDRESS_TREE_HEIGHT, merkle_tree::InstructionDataAddressAppendInputs,
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::{
    hash_chain::create_hash_chain_from_slice, instruction_data::compressed_proof::CompressedProof,
};
use light_hasher::{bigint::bigint_to_be_bytes_array, Poseidon};
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::batch_address_append::get_batch_address_append_circuit_inputs,
};
use light_sparse_merkle_tree::SparseMerkleTree;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::{error::ForesterUtilsError, rpc_pool::SolanaRpcPool, utils::wait_for_indexer};

const MAX_PHOTON_ELEMENTS_PER_CALL: usize = 500;

pub struct AddressUpdateConfig<R, I>
where
    R: Rpc + Send + Sync,
    I: Indexer + Send,
{
    pub rpc_pool: Arc<SolanaRpcPool<R>>,
    pub indexer: Arc<Mutex<I>>,
    pub merkle_tree_pubkey: Pubkey,
    pub prover_url: String,
    pub polling_interval: Duration,
    pub max_wait_time: Duration,
    pub ixs_per_tx: usize,
}

#[allow(clippy::too_many_arguments)]
fn stream_instruction_data<'a, I>(
    indexer: Arc<Mutex<I>>,
    merkle_tree_pubkey: Pubkey,
    prover_url: String,
    polling_interval: Duration,
    max_wait_time: Duration,
    leaves_hash_chains: Vec<[u8; 32]>,
    start_index: u64,
    zkp_batch_size: u16,
    mut current_root: [u8; 32],
    yield_batch_size: usize,
) -> impl Stream<Item = Result<Vec<InstructionDataAddressAppendInputs>, ForesterUtilsError>> + Send + 'a
where
    I: Indexer + Send + 'a,
{
    stream! {
        let proof_client = Arc::new(ProofClient::with_config(prover_url, polling_interval, max_wait_time));
        let max_zkp_batches_per_call = calculate_max_zkp_batches_per_call(zkp_batch_size);
        let total_chunks = leaves_hash_chains.len().div_ceil(max_zkp_batches_per_call);

        for chunk_idx in 0..total_chunks {
            let mut indexer_guard = indexer.lock().await;
            let chunk_start = chunk_idx * max_zkp_batches_per_call;
            let chunk_end = std::cmp::min(chunk_start + max_zkp_batches_per_call, leaves_hash_chains.len());
            let chunk_hash_chains = &leaves_hash_chains[chunk_start..chunk_end];

            let elements_for_chunk = chunk_hash_chains.len() * zkp_batch_size as usize;
            let processed_items_offset = chunk_start * zkp_batch_size as usize;

            let indexer_update_info = match indexer_guard
                .get_address_queue_with_proofs(
                    &merkle_tree_pubkey,
                    elements_for_chunk as u16,
                    Some(processed_items_offset as u64),
                    None,
                )
                .await {
                    Ok(info) => info,
                    Err(e) => {
                        yield Err(ForesterUtilsError::Indexer(format!("Failed to get address queue with proofs: {}", e)));
                        return;
                    }
                };

            if chunk_idx == 0 {
                if let Some(first_proof) = indexer_update_info.value.non_inclusion_proofs.first() {
                    if first_proof.root != current_root {
                        warn!("Indexer root does not match on-chain root");
                        yield Err(ForesterUtilsError::Indexer("Indexer root does not match on-chain root".into()));
                        return;
                    }
                } else {
                    yield Err(ForesterUtilsError::Indexer("No non-inclusion proofs found in indexer response".into()));
                    return;
                }
            }

            let (all_inputs, new_current_root) = match get_all_circuit_inputs_for_chunk(
                chunk_hash_chains,
                &indexer_update_info,
                zkp_batch_size,
                chunk_start,
                start_index,
                current_root,
            ) {
                Ok((inputs, new_root)) => (inputs, new_root),
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };
            current_root = new_current_root;

            info!("Generating {} ZK proofs with hybrid approach for chunk {}", all_inputs.len(), chunk_idx + 1);

            let mut futures_ordered = FuturesOrdered::new();
            let mut proof_buffer = Vec::new();
            let mut pending_count = 0;

            for (i, inputs) in all_inputs.into_iter().enumerate() {
                let client = Arc::clone(&proof_client);
                futures_ordered.push_back(async move {
                    let result = client.generate_batch_address_append_proof(inputs).await;
                    (i, result)
                });
                pending_count += 1;

                if pending_count >= yield_batch_size {
                    for _ in 0..yield_batch_size.min(pending_count) {
                        if let Some((idx, result)) = futures_ordered.next().await {
                            match result {
                                Ok((compressed_proof, new_root)) => {
                                    let instruction_data = InstructionDataAddressAppendInputs {
                                        new_root,
                                        compressed_proof: CompressedProof {
                                            a: compressed_proof.a,
                                            b: compressed_proof.b,
                                            c: compressed_proof.c,
                                        },
                                    };
                                    proof_buffer.push(instruction_data);
                                },
                                Err(e) => {
                                    error!("Address proof failed to generate at index {}: {:?}", idx, e);
                                    yield Err(ForesterUtilsError::Prover(format!(
                                        "Address proof generation failed at batch {} in chunk {}: {}",
                                        idx, chunk_idx, e
                                    )));
                                    return;
                                }
                            }
                            pending_count -= 1;
                        }
                    }

                    if !proof_buffer.is_empty() {
                        yield Ok(proof_buffer.clone());
                        proof_buffer.clear();
                    }
                }
            }

            while let Some((idx, result)) = futures_ordered.next().await {
                match result {
                    Ok((compressed_proof, new_root)) => {
                        let instruction_data = InstructionDataAddressAppendInputs {
                            new_root,
                            compressed_proof: CompressedProof {
                                a: compressed_proof.a,
                                b: compressed_proof.b,
                                c: compressed_proof.c,
                            },
                        };
                        proof_buffer.push(instruction_data);

                        if proof_buffer.len() >= yield_batch_size {
                            yield Ok(proof_buffer.clone());
                            proof_buffer.clear();
                        }
                    },
                    Err(e) => {
                        error!("Address proof failed to generate at index {}: {:?}", idx, e);
                        yield Err(ForesterUtilsError::Prover(format!(
                            "Address proof generation failed at batch {} in chunk {}: {}",
                            idx, chunk_idx, e
                        )));
                        return;
                    }
                }
            }

            if !proof_buffer.is_empty() {
                yield Ok(proof_buffer);
            }
        }
    }
}

fn calculate_max_zkp_batches_per_call(batch_size: u16) -> usize {
    std::cmp::max(1, MAX_PHOTON_ELEMENTS_PER_CALL / batch_size as usize)
}

fn get_all_circuit_inputs_for_chunk(
    chunk_hash_chains: &[[u8; 32]],
    indexer_update_info: &light_client::indexer::Response<
        light_client::indexer::BatchAddressUpdateIndexerResponse,
    >,
    batch_size: u16,
    chunk_start_idx: usize,
    global_start_index: u64,
    mut current_root: [u8; 32],
) -> Result<
    (
        Vec<light_prover_client::proof_types::batch_address_append::BatchAddressAppendInputs>,
        [u8; 32],
    ),
    ForesterUtilsError,
> {
    let subtrees_array: [[u8; 32]; DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize] =
        indexer_update_info
            .value
            .subtrees
            .clone()
            .try_into()
            .map_err(|_| {
                ForesterUtilsError::Prover("Failed to convert subtrees to array".into())
            })?;

    let mut sparse_merkle_tree =
        SparseMerkleTree::<Poseidon, { DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize }>::new(
            subtrees_array,
            global_start_index as usize + (chunk_start_idx * batch_size as usize),
        );

    let mut all_inputs = Vec::new();
    let mut changelog = Vec::new();
    let mut indexed_changelog = Vec::new();

    for (batch_idx, leaves_hash_chain) in chunk_hash_chains.iter().enumerate() {
        let start_idx = batch_idx * batch_size as usize;
        let end_idx = start_idx + batch_size as usize;
        let batch_addresses: Vec<[u8; 32]> = indexer_update_info.value.addresses
            [start_idx..end_idx]
            .iter()
            .map(|x| x.address)
            .collect();

        let mut low_element_values = Vec::new();
        let mut low_element_next_values = Vec::new();
        let mut low_element_indices = Vec::new();
        let mut low_element_next_indices = Vec::new();
        let mut low_element_proofs = Vec::new();

        for proof in &indexer_update_info.value.non_inclusion_proofs[start_idx..end_idx] {
            low_element_values.push(proof.low_address_value);
            low_element_indices.push(proof.low_address_index as usize);
            low_element_next_indices.push(proof.low_address_next_index as usize);
            low_element_next_values.push(proof.low_address_next_value);
            low_element_proofs.push(proof.low_address_proof.to_vec());
        }

        if create_hash_chain_from_slice(&batch_addresses)? != *leaves_hash_chain {
            return Err(ForesterUtilsError::Prover(
                "Addresses hash chain does not match".into(),
            ));
        }

        let adjusted_start_index = global_start_index as usize
            + (chunk_start_idx * batch_size as usize)
            + (batch_idx * batch_size as usize);

        let inputs = get_batch_address_append_circuit_inputs(
            adjusted_start_index,
            current_root,
            low_element_values,
            low_element_next_values,
            low_element_indices,
            low_element_next_indices,
            low_element_proofs,
            batch_addresses,
            &mut sparse_merkle_tree,
            *leaves_hash_chain,
            batch_size as usize,
            &mut changelog,
            &mut indexed_changelog,
        )
        .map_err(|e| ForesterUtilsError::Prover(format!("Failed to get circuit inputs: {}", e)))?;

        current_root = bigint_to_be_bytes_array::<32>(&inputs.new_root)?;
        all_inputs.push(inputs);
    }

    Ok((all_inputs, current_root))
}

pub async fn get_address_update_instruction_stream<'a, R, I>(
    config: AddressUpdateConfig<R, I>,
    merkle_tree_data: crate::ParsedMerkleTreeData,
) -> Result<
    (
        Pin<
            Box<
                dyn Stream<
                        Item = Result<Vec<InstructionDataAddressAppendInputs>, ForesterUtilsError>,
                    > + Send
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
    let rpc = config.rpc_pool.get_connection().await?;
    let indexer_guard = config.indexer.lock().await;
    wait_for_indexer(&*rpc, &*indexer_guard).await?;
    drop(rpc);
    drop(indexer_guard);

    let (current_root, leaves_hash_chains, start_index, zkp_batch_size) = (
        merkle_tree_data.current_root,
        merkle_tree_data.leaves_hash_chains,
        merkle_tree_data.next_index,
        merkle_tree_data.zkp_batch_size,
    );

    if leaves_hash_chains.is_empty() {
        debug!("No hash chains to process for address update, returning empty stream.");
        return Ok((Box::pin(futures::stream::empty()), zkp_batch_size));
    }

    let stream = stream_instruction_data(
        config.indexer,
        config.merkle_tree_pubkey,
        config.prover_url,
        config.polling_interval,
        config.max_wait_time,
        leaves_hash_chains,
        start_index,
        zkp_batch_size,
        current_root,
        config.ixs_per_tx,
    );

    Ok((Box::pin(stream), zkp_batch_size))
}
