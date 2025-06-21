use std::time::Duration;

use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use futures::future;
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
    merkle_tree::{
        BatchedMerkleTreeAccount, InstructionDataAddressAppendInputs,
        InstructionDataBatchNullifyInputs,
    },
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
use light_sparse_merkle_tree::{
    changelog::ChangelogEntry, indexed_changelog::IndexedChangelogEntry, SparseMerkleTree,
};
use tracing::{debug, error, info, warn};

use crate::{error::ForesterUtilsError, utils::wait_for_indexer};

const MAX_PHOTON_ELEMENTS_PER_CALL: usize = 500;

fn calculate_max_zkp_batches_per_call(batch_size: u16) -> usize {
    std::cmp::max(1, MAX_PHOTON_ELEMENTS_PER_CALL / batch_size as usize)
}

pub async fn create_batch_update_address_tree_instruction_data<R, I, F, Fut>(
    rpc: &mut R,
    indexer: &mut I,
    merkle_tree_pubkey: &Pubkey,
    prover_url: String,
    polling_interval: Duration,
    max_wait_time: Duration,
    instructions_per_tx: usize,
    mut tx_callback: F,
) -> Result<usize, ForesterUtilsError>
where
    R: Rpc,
    I: Indexer,
    // ===== THIS IS THE TYPE SIGNATURE FIX =====
    F: FnMut(Vec<InstructionDataAddressAppendInputs>, u16) -> Fut,
    Fut: std::future::Future<Output = Result<(), ForesterUtilsError>>,
{
    info!("Creating batch update address tree instruction data");

    let mut merkle_tree_account = rpc
        .get_account(*merkle_tree_pubkey)
        .await
        .map_err(|e| {
            error!("Failed to get account data from rpc: {:?}", e);
            ForesterUtilsError::Rpc("Failed to get account data".into())
        })?
        .unwrap();

    let (leaves_hash_chains, start_index, current_root, batch_size) = {
        let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &(*merkle_tree_pubkey).into(),
        )
        .unwrap();

        let full_batch_index = merkle_tree.queue_batches.pending_batch_index;
        let batch = &merkle_tree.queue_batches.batches[full_batch_index as usize];

        let mut hash_chains = Vec::new();
        let zkp_batch_index = batch.get_num_inserted_zkps();
        let current_zkp_batch_index = batch.get_current_zkp_batch_index();

        for i in zkp_batch_index..current_zkp_batch_index {
            hash_chains.push(merkle_tree.hash_chain_stores[full_batch_index as usize][i as usize]);
        }

        (
            hash_chains,
            merkle_tree.next_index,
            *merkle_tree.root_history.last().unwrap(),
            batch.zkp_batch_size as u16,
        )
    };

    if leaves_hash_chains.is_empty() {
        debug!("No hash chains to process");
        return Ok(0);
    }

    wait_for_indexer(rpc, indexer).await?;

    let max_zkp_batches_per_call = calculate_max_zkp_batches_per_call(batch_size);
    info!(
        "Processing {} ZK proof batches in chunks of {} (max {} elements per call)",
        leaves_hash_chains.len(),
        max_zkp_batches_per_call,
        MAX_PHOTON_ELEMENTS_PER_CALL
    );

    let proof_client = ProofClient::with_config(prover_url, polling_interval, max_wait_time);
    let mut pending_instructions: Vec<InstructionDataAddressAppendInputs> = Vec::new();
    let mut total_processed = 0;

    let total_chunks =
        (leaves_hash_chains.len() + max_zkp_batches_per_call - 1) / max_zkp_batches_per_call;

    for chunk_idx in 0..total_chunks {
        let chunk_start = chunk_idx * max_zkp_batches_per_call;
        let chunk_end = std::cmp::min(
            chunk_start + max_zkp_batches_per_call,
            leaves_hash_chains.len(),
        );
        let chunk_hash_chains = &leaves_hash_chains[chunk_start..chunk_end];

        let elements_for_chunk = chunk_hash_chains.len() * batch_size as usize;
        let processed_items_offset = chunk_start * batch_size as usize;

        let indexer_update_info = indexer
            .get_address_queue_with_proofs(
                merkle_tree_pubkey,
                elements_for_chunk as u16,
                Some(processed_items_offset as u64),
                None,
            )
            .await
            .map_err(|e| {
                error!("Failed to get batch address update info: {:?}", e);
                ForesterUtilsError::Indexer("Failed to get batch address update info".into())
            })?;

        if chunk_idx == 0 {
            let indexer_root = indexer_update_info
                .value
                .non_inclusion_proofs
                .first()
                .unwrap()
                .root;
            if indexer_root != current_root {
                warn!("Indexer root does not match on-chain root");
                return Err(ForesterUtilsError::Indexer(
                    "Indexer root does not match on-chain root".into(),
                ));
            }
        }

        let chunk_processed = process_hash_chain_chunk_streaming(
            chunk_hash_chains,
            &indexer_update_info,
            &proof_client,
            batch_size,
            chunk_start,
            start_index,
            current_root,
            instructions_per_tx,
            &mut pending_instructions,
            &mut tx_callback,
        )
        .await?;
        total_processed += chunk_processed;
    }

    if !pending_instructions.is_empty() {
        tx_callback(pending_instructions, batch_size).await?;
    }

    info!(
        "Successfully processed {} instruction batches across {} chunks",
        total_processed, total_chunks
    );
    Ok(total_processed)
}

async fn process_hash_chain_chunk_streaming<F, Fut>(
    chunk_hash_chains: &[[u8; 32]],
    indexer_update_info: &light_client::indexer::Response<
        light_client::indexer::BatchAddressUpdateIndexerResponse,
    >,
    proof_client: &ProofClient,
    batch_size: u16,
    chunk_start_idx: usize,
    global_start_index: u64,
    mut current_root: [u8; 32],
    instructions_per_tx: usize,
    pending_instructions: &mut Vec<InstructionDataAddressAppendInputs>,
    tx_callback: &mut F,
) -> Result<usize, ForesterUtilsError>
where
    F: FnMut(Vec<InstructionDataAddressAppendInputs>, u16) -> Fut,
    Fut: std::future::Future<Output = Result<(), ForesterUtilsError>>,
{
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

    info!("Generating {} ZK proofs concurrently", all_inputs.len());
    let proof_futures = all_inputs
        .into_iter()
        .map(|inputs| proof_client.generate_batch_address_append_proof(inputs));
    let proof_results = future::join_all(proof_futures).await;

    let mut processed_count = 0;
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
                pending_instructions.push(instruction_data);
                processed_count += 1;

                if pending_instructions.len() >= instructions_per_tx {
                    let tx_batch = pending_instructions.drain(0..instructions_per_tx).collect();
                    tx_callback(tx_batch, batch_size).await?;
                }
            }
            Err(e) => return Err(ForesterUtilsError::Prover(e.to_string())),
        }
    }

    Ok(processed_count)
}
