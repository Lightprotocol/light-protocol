use std::{pin::Pin, time::Duration}; // <-- Add Pin to imports

use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use async_stream::stream;
use futures::{future, stream::StreamExt, Stream};
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
    merkle_tree::{BatchedMerkleTreeAccount, InstructionDataAddressAppendInputs},
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::{
    hash_chain::create_hash_chain_from_slice, instruction_data::compressed_proof::CompressedProof,
};
use light_hasher::{bigint::bigint_to_be_bytes_array, Poseidon};
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::batch_address_append::{
        get_batch_address_append_circuit_inputs, BatchAddressAppendInputs,
    },
};
use light_sparse_merkle_tree::{
    changelog::ChangelogEntry, indexed_changelog::IndexedChangelogEntry, SparseMerkleTree,
};
use tracing::{debug, error, info, warn};

use crate::{error::ForesterUtilsError, utils::wait_for_indexer};

const MAX_PHOTON_ELEMENTS_PER_CALL: usize = 500;

// The return type is changed to reflect the Boxed and Pinned stream.
pub async fn get_address_update_stream<'a, R, I>(
    rpc: &'a mut R,
    indexer: &'a mut I,
    merkle_tree_pubkey: &'a Pubkey,
    prover_url: String,
    polling_interval: Duration,
    max_wait_time: Duration,
) -> Result<
    (
        Pin<
            Box<
                dyn Stream<Item = Result<InstructionDataAddressAppendInputs, ForesterUtilsError>>
                    + 'a,
            >,
        >,
        u16,
    ),
    ForesterUtilsError,
>
where
    R: Rpc + 'a,
    I: Indexer + 'a,
{
    info!("Fetching on-chain state to initialize address update stream");

    let mut merkle_tree_account = rpc
        .get_account(*merkle_tree_pubkey)
        .await?
        .ok_or_else(|| ForesterUtilsError::Rpc("Merkle tree account not found".into()))?;

    let (leaves_hash_chains, start_index, current_root, zkp_batch_size) = {
        let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &(*merkle_tree_pubkey).into(),
        )
        .map_err(|e| {
            ForesterUtilsError::AccountZeroCopy(format!("Failed to parse merkle tree: {}", e))
        })?;

        let full_batch_index = merkle_tree.queue_batches.pending_batch_index;
        let batch = &merkle_tree.queue_batches.batches[full_batch_index as usize];
        let mut hash_chains = Vec::new();
        let zkp_batch_index = batch.get_num_inserted_zkps();
        let current_zkp_batch_index = batch.get_current_zkp_batch_index();

        for i in zkp_batch_index..current_zkp_batch_index {
            hash_chains.push(merkle_tree.hash_chain_stores[full_batch_index as usize][i as usize]);
        }

        let root = *merkle_tree.root_history.last().ok_or_else(|| {
            ForesterUtilsError::Prover("Merkle tree root history is empty".into())
        })?;

        (
            hash_chains,
            merkle_tree.next_index,
            root,
            batch.zkp_batch_size as u16,
        )
    };

    if leaves_hash_chains.is_empty() {
        debug!("No hash chains to process, returning empty stream.");
        // FIX #1: Box and pin the empty stream.
        return Ok((Box::pin(futures::stream::empty()), zkp_batch_size));
    }

    wait_for_indexer(rpc, indexer).await?;

    let stream = stream_instruction_data(
        indexer,
        merkle_tree_pubkey,
        prover_url,
        polling_interval,
        max_wait_time,
        leaves_hash_chains,
        start_index,
        current_root,
        zkp_batch_size,
    );

    // FIX #2: Box and pin the instruction data stream.
    Ok((Box::pin(stream), zkp_batch_size))
}

// Helper function's return type also changes to match.
fn stream_instruction_data<'a, I>(
    indexer: &'a mut I,
    merkle_tree_pubkey: &'a Pubkey,
    prover_url: String,
    polling_interval: Duration,
    max_wait_time: Duration,
    leaves_hash_chains: Vec<[u8; 32]>,
    start_index: u64,
    mut current_root: [u8; 32],
    zkp_batch_size: u16,
) -> impl Stream<Item = Result<InstructionDataAddressAppendInputs, ForesterUtilsError>> + 'a
where
    I: Indexer + 'a,
{
    stream! {
        let proof_client = ProofClient::with_config(prover_url, polling_interval, max_wait_time);
        let max_zkp_batches_per_call = calculate_max_zkp_batches_per_call(zkp_batch_size);
        let total_chunks = (leaves_hash_chains.len() + max_zkp_batches_per_call - 1) / max_zkp_batches_per_call;

        for chunk_idx in 0..total_chunks {
            let chunk_start = chunk_idx * max_zkp_batches_per_call;
            let chunk_end = std::cmp::min(chunk_start + max_zkp_batches_per_call, leaves_hash_chains.len());
            let chunk_hash_chains = &leaves_hash_chains[chunk_start..chunk_end];

            let elements_for_chunk = chunk_hash_chains.len() * zkp_batch_size as usize;
            let processed_items_offset = chunk_start * zkp_batch_size as usize;

            let indexer_update_info = match indexer
                .get_address_queue_with_proofs(
                    merkle_tree_pubkey,
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

            info!("Generating {} ZK proofs concurrently for chunk {}", all_inputs.len(), chunk_idx + 1);
            let proof_futures = all_inputs
                .into_iter()
                .map(|inputs| proof_client.generate_batch_address_append_proof(inputs));

            let proof_results = future::join_all(proof_futures).await;

            for proof_result in proof_results {
                match proof_result {
                    Ok((compressed_proof, new_root)) => {
                        let instruction_data = InstructionDataAddressAppendInputs {
                            new_root,
                            compressed_proof: CompressedProof {
                                a: compressed_proof.a,
                                b: compressed_proof.b,
                                c: compressed_proof.c,
                            },
                        };
                        yield Ok(instruction_data);
                    }
                    Err(e) => {
                        error!("A proof failed to generate: {:?}", e);
                        yield Err(ForesterUtilsError::Prover(e.to_string()));
                        return;
                    }
                }
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
) -> Result<(Vec<BatchAddressAppendInputs>, [u8; 32]), ForesterUtilsError> {
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

    let all_addresses: Vec<[u8; 32]> = indexer_update_info
        .value
        .addresses
        .iter()
        .map(|x| x.address)
        .collect();

    let mut all_inputs = Vec::new();
    let mut changelog = Vec::new();
    let mut indexed_changelog = Vec::new();

    for (batch_idx, leaves_hash_chain) in chunk_hash_chains.iter().enumerate() {
        let start_addr_idx = batch_idx * batch_size as usize;
        let end_addr_idx = start_addr_idx + batch_size as usize;
        if end_addr_idx > all_addresses.len() {
            return Err(ForesterUtilsError::Indexer(
                "Not enough addresses from indexer".into(),
            ));
        }
        let batch_addresses = all_addresses[start_addr_idx..end_addr_idx].to_vec();

        let start_proof_idx = batch_idx * batch_size as usize;
        let end_proof_idx = start_proof_idx + batch_size as usize;
        if end_proof_idx > indexer_update_info.value.non_inclusion_proofs.len() {
            return Err(ForesterUtilsError::Indexer(
                "Not enough proofs from indexer".into(),
            ));
        }
        let batch_proofs =
            &indexer_update_info.value.non_inclusion_proofs[start_proof_idx..end_proof_idx];

        let mut low_element_values = Vec::new();
        let mut low_element_indices = Vec::new();
        let mut low_element_next_indices = Vec::new();
        let mut low_element_next_values = Vec::new();
        let mut low_element_proofs = Vec::new();

        for proof in batch_proofs {
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

        current_root = bigint_to_be_bytes_array::<32>(&inputs.new_root).unwrap();
        all_inputs.push(inputs);
    }

    Ok((all_inputs, current_root))
}
