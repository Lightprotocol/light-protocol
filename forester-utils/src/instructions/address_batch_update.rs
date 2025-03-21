use crate::error::ForesterUtilsError;
use crate::utils::{create_reference_address_tree, wait_for_indexer};
use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_ADDRESS_TREE_HEIGHT,
    merkle_tree::{BatchedMerkleTreeAccount, InstructionDataBatchNullifyInputs},
};
use light_client::{indexer::Indexer, rpc::RpcConnection};
use light_compressed_account::hash_chain::create_hash_chain_from_slice;
use light_compressed_account::{
    bigint::bigint_to_be_bytes_array, instruction_data::compressed_proof::CompressedProof,
};
use light_hasher::Poseidon;
use light_merkle_tree_reference::sparse_merkle_tree::SparseMerkleTree;
use light_prover_client::{
    batch_address_append::get_batch_address_append_circuit_inputs,
    gnark::{
        batch_address_append_json_formatter::to_json,
        constants::{PROVE_PATH, SERVER_ADDRESS},
        proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    },
};
use reqwest::Client;
use tracing::{debug, error, warn};

pub async fn create_batch_update_address_tree_instruction_data<R, I>(
    rpc: &mut R,
    indexer: &mut I,
    merkle_tree_pubkey: &Pubkey,
) -> Result<(InstructionDataBatchNullifyInputs, usize), ForesterUtilsError>
where
    R: RpcConnection,
    I: Indexer<R>,
{
    let mut merkle_tree_account = rpc.get_account(*merkle_tree_pubkey).await
        .map_err(|e| {
            error!(
                "create_batch_update_address_tree_instruction_data: failed to get account data from rpc: {:?}",
                e
            );
            ForesterUtilsError::Rpc("Failed to get account data".into())
        })?
        .unwrap();

    let (leaves_hash_chain, start_index, current_root, batch_size) = {
        let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &merkle_tree_pubkey.into(),
        )
        .unwrap();

        let full_batch_index = merkle_tree.queue_batches.pending_batch_index;
        let batch = &merkle_tree.queue_batches.batches[full_batch_index as usize];
        let zkp_batch_index = batch.get_num_inserted_zkps();
        println!("full batch index: {}, zkp batch index: {}", full_batch_index, zkp_batch_index);
        let leaves_hash_chain =
            merkle_tree.hash_chain_stores[full_batch_index as usize][zkp_batch_index as usize];
        let start_index = merkle_tree.next_index;
        let current_root = *merkle_tree.root_history.last().unwrap();
        let zkp_batch_size = batch.zkp_batch_size as u16;

        (leaves_hash_chain, start_index, current_root, zkp_batch_size)
    };

    wait_for_indexer(rpc, indexer).await.unwrap();

    let indexer_update_info = indexer
        .get_batch_address_update_info(merkle_tree_pubkey, batch_size)
        .await
        .map_err(|_| {
            ForesterUtilsError::Indexer("Failed to get batch address update info".into())
        })?;

    let indexer_root = indexer_update_info.non_inclusion_proofs.first().unwrap().root;
    assert_eq!(indexer_root, current_root);

    let batch_size = indexer_update_info.addresses.len();

    let mut low_element_values = Vec::new();
    let mut low_element_indices = Vec::new();
    let mut low_element_next_indices = Vec::new();
    let mut low_element_next_values = Vec::new();
    let mut low_element_proofs: Vec<Vec<[u8; 32]>> = Vec::new();

    for non_inclusion_proof in &indexer_update_info.non_inclusion_proofs {
        low_element_values.push(non_inclusion_proof.low_address_value);
        low_element_indices.push(non_inclusion_proof.low_address_index as usize);
        low_element_next_indices.push(non_inclusion_proof.low_address_next_index as usize);
        low_element_next_values.push(non_inclusion_proof.low_address_next_value);
        low_element_proofs.push(non_inclusion_proof.low_address_proof.to_vec());
    }

    let addresses = indexer_update_info
        .addresses
        .iter()
        .map(|x| x.address)
        .collect::<Vec<[u8; 32]>>();

    let addresses_hashchain = create_hash_chain_from_slice(addresses.as_slice()).unwrap();

    warn!("create_batch_update_address_tree_instruction_data: addresses hash chain does not match leaves hash chain");
    warn!("addresses hash chain: {:?}", addresses_hashchain);
    warn!("leaves hash chain: {:?}", leaves_hash_chain);
    warn!("start index: {}", start_index);
    warn!("indexer update info start index: {}", indexer_update_info.batch_start_index);
    for (i, address) in addresses.iter().enumerate() {
        warn!("address {}: {:?}", i, address);
    }

    if addresses_hashchain != leaves_hash_chain {
        panic!("Addresses hash chain does not match leaves hash chain");
    }

    let subtrees: [[u8; 32]; DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize] = indexer_update_info
        .subtrees
        .try_into()
        .map_err(|_| ForesterUtilsError::Prover("Failed to convert subtrees to array".into()))?;
    let mut sparse_merkle_tree = SparseMerkleTree::<Poseidon, { DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize }>::new(<[[u8; 32]; DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize]>::try_from(subtrees).unwrap(), start_index as usize);
    let ref_tree = create_reference_address_tree(
        merkle_tree_pubkey,
        0,
        start_index-1
    );
    let inputs =
        get_batch_address_append_circuit_inputs::<{ DEFAULT_BATCH_ADDRESS_TREE_HEIGHT as usize }>(
            start_index as usize,
            current_root,
            low_element_values,
            low_element_next_values,
            low_element_indices,
            low_element_next_indices,
            low_element_proofs,
            addresses,
            &mut sparse_merkle_tree,
            leaves_hash_chain,
            batch_size,
            Some(ref_tree),
        )
        .map_err(|e| {
            error!(
            "create_batch_update_address_tree_instruction_data: failed to get circuit inputs: {:?}",
            e
        );
            ForesterUtilsError::Prover("Failed to get circuit inputs".into())
        })?;

    let client = Client::new();
    let new_root = bigint_to_be_bytes_array::<32>(&inputs.new_root).unwrap();
    let inputs = to_json(&inputs);

    debug!("prover inputs: {}", inputs);

    let response_result = client
        .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(inputs)
        .send()
        .await
        .expect("Failed to execute request.");

    if response_result.status().is_success() {
        let body = response_result.text().await.unwrap();
        let proof_json = deserialize_gnark_proof_json(&body).unwrap();
        let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
        let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);
        let instruction_data = InstructionDataBatchNullifyInputs {
            new_root,
            compressed_proof: CompressedProof {
                a: proof_a,
                b: proof_b,
                c: proof_c,
            },
        };
        Ok((instruction_data, batch_size))
    } else {
        Err(ForesterUtilsError::Prover(
            "Prover failed to generate proof".to_string(),
        ))
    }
}
