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
use light_client::{
    indexer::{AddressQueueDataV2, Indexer, QueueElementsV2Options},
    rpc::Rpc,
};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_hasher::{bigint::bigint_to_be_bytes_array, Poseidon};
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::batch_address_append::get_batch_address_append_circuit_inputs,
};
use light_sparse_merkle_tree::SparseMerkleTree;
use tracing::{debug, error, info};

use crate::{error::ForesterUtilsError, rpc_pool::SolanaRpcPool};

const MAX_PHOTON_ELEMENTS_PER_CALL: usize = 500;
const MAX_PROOFS_PER_TX: usize = 3;

pub struct AddressUpdateConfig<R: Rpc> {
    pub rpc_pool: Arc<SolanaRpcPool<R>>,
    pub merkle_tree_pubkey: Pubkey,
    pub prover_url: String,
    pub prover_api_key: Option<String>,
    pub polling_interval: Duration,
    pub max_wait_time: Duration,
}

#[allow(clippy::too_many_arguments)]
async fn stream_instruction_data<'a, R: Rpc>(
    rpc_pool: Arc<SolanaRpcPool<R>>,
    merkle_tree_pubkey: Pubkey,
    prover_url: String,
    prover_api_key: Option<String>,
    polling_interval: Duration,
    max_wait_time: Duration,
    leaves_hash_chains: Vec<[u8; 32]>,
    start_index: u64,
    zkp_batch_size: u16,
    mut current_root: [u8; 32],
) -> impl Stream<Item = Result<Vec<InstructionDataAddressAppendInputs>, ForesterUtilsError>> + Send + 'a
{
    stream! {
        let proof_client = Arc::new(ProofClient::with_config(prover_url, polling_interval, max_wait_time, prover_api_key));
        let max_zkp_batches_per_call = calculate_max_zkp_batches_per_call(zkp_batch_size);
        let total_chunks = leaves_hash_chains.len().div_ceil(max_zkp_batches_per_call);

        let mut next_queue_index: Option<u64> = None;

        for chunk_idx in 0..total_chunks {
            let chunk_start = chunk_idx * max_zkp_batches_per_call;
            let chunk_end = std::cmp::min(chunk_start + max_zkp_batches_per_call, leaves_hash_chains.len());
            let chunk_hash_chains = &leaves_hash_chains[chunk_start..chunk_end];

            let elements_for_chunk = chunk_hash_chains.len() * zkp_batch_size as usize;

            let queue_elements_response = {
                let mut connection = rpc_pool.get_connection().await?;
                let indexer = connection.indexer_mut()?;
                let mut options = QueueElementsV2Options::default().with_address_queue(
                    next_queue_index,
                    Some(elements_for_chunk as u16),
                );
                options.address_queue_zkp_batch_size =  Some(zkp_batch_size);

                match indexer
                    .get_queue_elements_v2(merkle_tree_pubkey.to_bytes(), options, None)
                    .await
                {
                    Ok(info) => info,
                    Err(e) => {
                        yield Err(ForesterUtilsError::Indexer(format!(
                            "Failed to get address queue: {}",
                            e
                        )));
                        return;
                    }
                }
            };

            let address_queue = queue_elements_response
                .value
                .address_queue
                .ok_or_else(|| {
                    ForesterUtilsError::Indexer(
                        "Indexer response missing address queue data".into(),
                    )
                })?;

            debug!(
                "chunk {}: received {} addresses, batch_start_index={}, first_queue_index={:?}, last_queue_index={:?}",
                chunk_idx,
                address_queue.addresses.len(),
                address_queue.start_index,
                address_queue.queue_indices.first(),
                address_queue.queue_indices.last()
            );

            if let Some(last_index) = address_queue.queue_indices.last() {
                next_queue_index = Some(last_index + 1);
                debug!(
                    "Setting next_queue_index={} for chunk {}",
                    next_queue_index.unwrap(),
                    chunk_idx + 1
                );
            }

            if chunk_idx == 0 {
                if address_queue.initial_root != current_root {
                    yield Err(ForesterUtilsError::Indexer(
                        "Indexer root does not match on-chain root".into(),
                    ));
                    return;
                }
            }

            let (all_inputs, new_current_root) = match get_all_circuit_inputs_for_chunk(
                chunk_hash_chains,
                &address_queue,
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

                if pending_count >= MAX_PROOFS_PER_TX {
                    for _ in 0..MAX_PROOFS_PER_TX.min(pending_count) {
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

                        if proof_buffer.len() >= MAX_PROOFS_PER_TX {
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
    address_queue: &AddressQueueDataV2,
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
        address_queue.subtrees.clone().try_into().map_err(|_| {
            ForesterUtilsError::Prover("Failed to convert subtrees to array".into())
        })?;

    let mut sparse_merkle_tree =
        SparseMerkleTree::<Poseidon, { DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize }>::new(
            subtrees_array,
            global_start_index as usize + (chunk_start_idx * batch_size as usize),
        );

    if address_queue.low_element_proofs.len() != address_queue.low_element_indices.len() {
        return Err(ForesterUtilsError::Indexer(format!(
            "Address queue missing proofs: expected {} entries, got {}",
            address_queue.low_element_indices.len(),
            address_queue.low_element_proofs.len()
        )));
    }
    let low_element_proofs = address_queue.low_element_proofs.clone();

    let mut all_inputs = Vec::new();
    let mut changelog = Vec::new();
    let mut indexed_changelog = Vec::new();

    for (batch_idx, leaves_hash_chain) in chunk_hash_chains.iter().enumerate() {
        let start_idx = batch_idx * batch_size as usize;
        let end_idx = start_idx + batch_size as usize;

        let addresses_len = address_queue.addresses.len();
        if start_idx >= addresses_len {
            return Err(ForesterUtilsError::Indexer(format!(
                "Insufficient addresses: batch {} requires start_idx {} but only {} addresses available",
                batch_idx, start_idx, addresses_len
            )));
        }
        let safe_end_idx = std::cmp::min(end_idx, addresses_len);
        if safe_end_idx - start_idx != batch_size as usize {
            return Err(ForesterUtilsError::Indexer(format!(
                "Insufficient addresses: batch {} requires {} addresses (indices {}..{}) but only {} available",
                batch_idx, batch_size, start_idx, end_idx, safe_end_idx - start_idx
            )));
        }

        let batch_addresses: Vec<[u8; 32]> =
            address_queue.addresses[start_idx..safe_end_idx].to_vec();

        let proofs_len = address_queue.low_element_indices.len();
        if start_idx >= proofs_len {
            return Err(ForesterUtilsError::Indexer(format!(
                "Insufficient non-inclusion proofs: batch {} requires start_idx {} but only {} proofs available",
                batch_idx, start_idx, proofs_len
            )));
        }
        let safe_proofs_end_idx = std::cmp::min(end_idx, proofs_len);
        if safe_proofs_end_idx - start_idx != batch_size as usize {
            return Err(ForesterUtilsError::Indexer(format!(
                "Insufficient non-inclusion proofs: batch {} requires {} proofs (indices {}..{}) but only {} available",
                batch_idx, batch_size, start_idx, end_idx, safe_proofs_end_idx - start_idx
            )));
        }

        let low_element_values = address_queue.low_element_values[start_idx..end_idx].to_vec();
        let low_element_next_values =
            address_queue.low_element_next_values[start_idx..end_idx].to_vec();
        let low_element_indices = address_queue.low_element_indices[start_idx..end_idx]
            .iter()
            .map(|idx| *idx as usize)
            .collect::<Vec<_>>();
        let low_element_next_indices = address_queue.low_element_next_indices[start_idx..end_idx]
            .iter()
            .map(|idx| *idx as usize)
            .collect::<Vec<_>>();
        let low_element_proofs = low_element_proofs[start_idx..end_idx]
            .iter()
            .cloned()
            .collect::<Vec<_>>();

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

pub async fn get_address_update_instruction_stream<'a, R: Rpc>(
    config: AddressUpdateConfig<R>,
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
> {
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
        config.rpc_pool,
        config.merkle_tree_pubkey,
        config.prover_url,
        config.prover_api_key,
        config.polling_interval,
        config.max_wait_time,
        leaves_hash_chains,
        start_index,
        zkp_batch_size,
        current_root,
    )
    .await;

    Ok((Box::pin(stream), zkp_batch_size))
}
