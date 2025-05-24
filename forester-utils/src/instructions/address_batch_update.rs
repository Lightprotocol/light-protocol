use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use futures::future;
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
    merkle_tree::{
        BatchedMerkleTreeAccount, InstructionDataAddressAppendInputs,
        InstructionDataBatchNullifyInputs,
    },
};
use light_client::{indexer::Indexer, rpc::RpcConnection};
use light_compressed_account::hash_chain::create_hash_chain_from_slice;
use light_concurrent_merkle_tree::changelog::ChangelogEntry;
use light_hasher::{bigint::bigint_to_be_bytes_array, Poseidon};
use light_indexed_array::changelog::IndexedChangelogEntry;
use light_merkle_tree_reference::sparse_merkle_tree::SparseMerkleTree;
use light_prover_client::{
    batch_address_append::get_batch_address_append_circuit_inputs, proof_client::ProofClient,
};
use tracing::{debug, error, info, warn};

use crate::{error::ForesterUtilsError, utils::wait_for_indexer};

pub async fn create_batch_update_address_tree_instruction_data<R, I>(
    rpc: &mut R,
    indexer: &mut I,
    merkle_tree_pubkey: &Pubkey,
) -> Result<(Vec<InstructionDataBatchNullifyInputs>, u16), ForesterUtilsError>
where
    R: RpcConnection,
    I: Indexer,
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
            &merkle_tree_pubkey.into(),
        )
        .unwrap();

        let full_batch_index = merkle_tree.queue_batches.pending_batch_index;
        let batch = &merkle_tree.queue_batches.batches[full_batch_index as usize];

        let mut hash_chains = Vec::new();
        let zkp_batch_index = batch.get_num_inserted_zkps();
        let current_zkp_batch_index = batch.get_current_zkp_batch_index();

        debug!(
            "Full batch index: {}, inserted ZKPs: {}, current ZKP index: {}, ready for insertion: {}",
            full_batch_index, zkp_batch_index, current_zkp_batch_index, current_zkp_batch_index - zkp_batch_index
        );

        for i in zkp_batch_index..current_zkp_batch_index {
            hash_chains.push(merkle_tree.hash_chain_stores[full_batch_index as usize][i as usize]);
        }

        let start_index = merkle_tree.next_index;
        let current_root = *merkle_tree.root_history.last().unwrap();
        let zkp_batch_size = batch.zkp_batch_size as u16;

        (hash_chains, start_index, current_root, zkp_batch_size)
    };

    if leaves_hash_chains.is_empty() {
        debug!("No hash chains to process");
        return Ok((Vec::new(), batch_size));
    }

    wait_for_indexer(rpc, indexer).await?;

    let total_elements = batch_size as usize * leaves_hash_chains.len();
    debug!("Requesting {} total elements from indexer", total_elements);

    let indexer_update_info = indexer
        .get_address_queue_with_proofs(merkle_tree_pubkey, total_elements as u16)
        .await
        .map_err(|e| {
            error!("Failed to get batch address update info: {:?}", e);
            ForesterUtilsError::Indexer("Failed to get batch address update info".into())
        })?;
    debug!("indexer_update_info {:?}", indexer_update_info);
    let indexer_root = indexer_update_info
        .non_inclusion_proofs
        .first()
        .unwrap()
        .root;

    if indexer_root != current_root {
        warn!("Indexer root does not match on-chain root");
        warn!("Indexer root: {:?}", indexer_root);
        warn!("On-chain root: {:?}", current_root);

        return Err(ForesterUtilsError::Indexer(
            "Indexer root does not match on-chain root".into(),
        ));
    }

    let subtrees_array: [[u8; 32]; DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize] =
        indexer_update_info
            .subtrees
            .clone()
            .try_into()
            .map_err(|_| {
                ForesterUtilsError::Prover("Failed to convert subtrees to array".into())
            })?;

    let mut sparse_merkle_tree = SparseMerkleTree::<
        Poseidon,
        { DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize },
    >::new(subtrees_array, start_index as usize);

    let all_addresses = indexer_update_info
        .addresses
        .iter()
        .map(|x| x.address)
        .collect::<Vec<[u8; 32]>>();

    debug!("Got {} addresses from indexer", all_addresses.len());

    let mut all_inputs = Vec::new();
    let mut current_root = current_root;

    let mut changelog: Vec<ChangelogEntry<{ DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize }>> =
        Vec::new();
    let mut indexed_changelog: Vec<
        IndexedChangelogEntry<usize, { DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize }>,
    > = Vec::new();

    for (batch_idx, leaves_hash_chain) in leaves_hash_chains.iter().enumerate() {
        debug!(
            "Preparing circuit inputs for batch {} with root {:?}",
            batch_idx, current_root
        );

        let start_addr_idx = batch_idx * batch_size as usize;
        let end_addr_idx = start_addr_idx + batch_size as usize;

        if end_addr_idx > all_addresses.len() {
            error!(
                "Not enough addresses from indexer. Expected at least {}, got {}",
                end_addr_idx,
                all_addresses.len()
            );
            return Err(ForesterUtilsError::Indexer(
                "Not enough addresses from indexer".into(),
            ));
        }

        let batch_addresses = all_addresses[start_addr_idx..end_addr_idx].to_vec();

        let start_proof_idx = batch_idx * batch_size as usize;
        let end_proof_idx = start_proof_idx + batch_size as usize;

        if end_proof_idx > indexer_update_info.non_inclusion_proofs.len() {
            error!(
                "Not enough proofs from indexer. Expected at least {}, got {}",
                end_proof_idx,
                indexer_update_info.non_inclusion_proofs.len()
            );
            return Err(ForesterUtilsError::Indexer(
                "Not enough proofs from indexer".into(),
            ));
        }

        let batch_proofs =
            &indexer_update_info.non_inclusion_proofs[start_proof_idx..end_proof_idx];

        let mut low_element_values = Vec::new();
        let mut low_element_indices = Vec::new();
        let mut low_element_next_indices = Vec::new();
        let mut low_element_next_values = Vec::new();
        let mut low_element_proofs: Vec<Vec<[u8; 32]>> = Vec::new();

        for proof in batch_proofs {
            low_element_values.push(proof.low_address_value);
            low_element_indices.push(proof.low_address_index as usize);
            low_element_next_indices.push(proof.low_address_next_index as usize);
            low_element_next_values.push(proof.low_address_next_value);
            low_element_proofs.push(proof.low_address_proof.to_vec());
        }

        let addresses_hashchain = create_hash_chain_from_slice(batch_addresses.as_slice())
            .map_err(|e| {
                error!("Failed to create hash chain from addresses: {:?}", e);
                ForesterUtilsError::Prover("Failed to create hash chain from addresses".into())
            })?;

        if addresses_hashchain != *leaves_hash_chain {
            error!(
                "Addresses hash chain does not match leaves hash chain for batch {}",
                batch_idx
            );
            error!("Addresses hash chain: {:?}", addresses_hashchain);
            error!("Leaves hash chain: {:?}", leaves_hash_chain);
            return Err(ForesterUtilsError::Prover(
                "Addresses hash chain does not match leaves hash chain".into(),
            ));
        }

        let adjusted_start_index = start_index as usize + (batch_idx * batch_size as usize);

        debug!(
            "Batch {} using root {:?}, start index {}",
            batch_idx, current_root, adjusted_start_index
        );

        let inputs = get_batch_address_append_circuit_inputs::<
            { DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize },
        >(
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
        .map_err(|e| {
            error!(
                "Failed to get circuit inputs for batch {}: {:?}",
                batch_idx, e
            );
            ForesterUtilsError::Prover(format!(
                "Failed to get circuit inputs for batch {}: {}",
                batch_idx, e
            ))
        })?;

        current_root = bigint_to_be_bytes_array::<32>(&inputs.new_root).unwrap();
        debug!("Updated root after batch {}: {:?}", batch_idx, current_root);
        all_inputs.push(inputs);
    }

    info!("Generating {} ZK proofs asynchronously", all_inputs.len());
    let proof_client = ProofClient::local();
    let proof_futures = all_inputs
        .into_iter()
        .map(|inputs| proof_client.generate_batch_address_append_proof(inputs));
    let proof_results = future::join_all(proof_futures).await;

    let mut instruction_data_vec = Vec::new();
    for (i, proof_result) in proof_results.into_iter().enumerate() {
        match proof_result {
            Ok((compressed_proof, new_root)) => {
                debug!("Successfully generated proof for batch {}", i);
                instruction_data_vec.push(InstructionDataAddressAppendInputs {
                    new_root,
                    compressed_proof,
                });
            }
            Err(e) => {
                error!("Failed to generate proof for batch {}: {:?}", i, e);
                return Err(ForesterUtilsError::Prover(e.to_string()));
            }
        }
    }

    info!(
        "Successfully generated {} instruction data entries",
        instruction_data_vec.len()
    );
    Ok((instruction_data_vec, batch_size))
}
