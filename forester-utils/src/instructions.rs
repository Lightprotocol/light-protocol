use light_batched_merkle_tree::{
    constants::{DEFAULT_BATCH_ADDRESS_TREE_HEIGHT, DEFAULT_BATCH_STATE_TREE_HEIGHT},
    merkle_tree::{
        BatchedMerkleTreeAccount, InstructionDataBatchAppendInputs,
        InstructionDataBatchNullifyInputs,
    },
    queue::BatchedQueueAccount,
};
use light_client::{indexer::Indexer, rpc::RpcConnection};
use light_hasher::{Hasher, Poseidon};
use light_prover_client::{
    batch_address_append::get_batch_address_append_circuit_inputs,
    batch_append_with_proofs::get_batch_append_with_proofs_inputs,
    batch_update::get_batch_update_inputs,
    gnark::{
        batch_address_append_json_formatter::to_json,
        batch_append_with_proofs_json_formatter::BatchAppendWithProofsInputsJson,
        batch_update_json_formatter::update_inputs_string,
        constants::{PROVE_PATH, SERVER_ADDRESS},
        proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    },
};
use light_utils::{
    bigint::bigint_to_be_bytes_array, instruction::compressed_proof::CompressedProof,
};
use log::{error, info};
use reqwest::Client;
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ForesterUtilsError {
    #[error("parse error: {0:?}")]
    ParseError(String),
    #[error("prover error: {0:?}")]
    ProverError(String),
    #[error("rpc error: {0:?}")]
    RpcError(String),
    #[error("indexer error: {0:?}")]
    IndexerError(String),
}

pub async fn create_batch_update_address_tree_instruction_data<R, I>(
    rpc: &mut R,
    indexer: &mut I,
    merkle_tree_pubkey: Pubkey,
) -> Result<(InstructionDataBatchNullifyInputs, usize), ForesterUtilsError>
where
    R: RpcConnection,
    I: Indexer<R>,
{
    let mut merkle_tree_account = rpc.get_account(merkle_tree_pubkey).await
        .map_err(|e| {
            error!(
                "create_batch_update_address_tree_instruction_data: failed to get account data from rpc: {:?}",
                e
            );
            ForesterUtilsError::RpcError("Failed to get account data".into())
        })?
        .unwrap();

    let (leaves_hashchain, start_index, current_root, batch_size, full_batch_index) = {
        let merkle_tree =
            BatchedMerkleTreeAccount::address_from_bytes(merkle_tree_account.data.as_mut_slice())
                .unwrap();

        let full_batch_index = merkle_tree.queue_metadata.next_full_batch_index;
        let batch = &merkle_tree.queue_metadata.batches[full_batch_index as usize];
        let zkp_batch_index = batch.get_num_inserted_zkps();
        let leaves_hashchain =
            merkle_tree.hash_chain_stores[full_batch_index as usize][zkp_batch_index as usize];
        let start_index = merkle_tree.next_index;
        let current_root = *merkle_tree.root_history.last().unwrap();
        let batch_size = batch.zkp_batch_size as usize;

        (
            leaves_hashchain,
            start_index,
            current_root,
            batch_size,
            full_batch_index,
        )
    };

    let batch_start_index = indexer
        .get_address_merkle_trees()
        .iter()
        .find(|x| x.accounts.merkle_tree == merkle_tree_pubkey)
        .unwrap()
        .merkle_tree
        .merkle_tree
        .rightmost_index;

    let addresses = indexer
        .get_queue_elements(
            merkle_tree_pubkey.to_bytes(),
            full_batch_index,
            0,
            batch_size as u64,
        )
        .await
        .map_err(|e| {
            error!(
                "create_batch_update_address_tree_instruction_data: failed to get queue elements from indexer: {:?}",
                e
            );
            ForesterUtilsError::IndexerError("Failed to get queue elements".into())
        })?;

    let batch_size = addresses.len();

    // Get proof info after addresses are retrieved
    let non_inclusion_proofs = indexer
        .get_multiple_new_address_proofs_h40(
            merkle_tree_pubkey.to_bytes(),
            addresses.clone(),
        )
        .await
        .map_err(|e| {
            error!(
                "create_batch_update_address_tree_instruction_data: failed to get get_multiple_new_address_proofs_full from indexer: {:?}",
                e
            );
            ForesterUtilsError::IndexerError("Failed to get get_multiple_new_address_proofs_full".into())
        })?;

    let mut low_element_values = Vec::new();
    let mut low_element_indices = Vec::new();
    let mut low_element_next_indices = Vec::new();
    let mut low_element_next_values = Vec::new();
    let mut low_element_proofs: Vec<Vec<[u8; 32]>> = Vec::new();

    for non_inclusion_proof in &non_inclusion_proofs {
        low_element_values.push(non_inclusion_proof.low_address_value);
        low_element_indices.push(non_inclusion_proof.low_address_index as usize);
        low_element_next_indices.push(non_inclusion_proof.low_address_next_index as usize);
        low_element_next_values.push(non_inclusion_proof.low_address_next_value);
        low_element_proofs.push(non_inclusion_proof.low_address_proof.to_vec());
    }

    let subtrees = indexer
        .get_subtrees(merkle_tree_pubkey.to_bytes())
        .await
        .map_err(|e| {
            error!(
                "create_batch_update_address_tree_instruction_data: failed to get subtrees from indexer: {:?}",
                e
            );
            ForesterUtilsError::IndexerError("Failed to get subtrees".into())
        })?
        .try_into()
        .unwrap();

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
            subtrees,
            leaves_hashchain,
            batch_start_index,
            batch_size,
        )
        .map_err(|e| {
            error!(
            "create_batch_update_address_tree_instruction_data: failed to get circuit inputs: {:?}",
            e
        );
            ForesterUtilsError::ProverError("Failed to get circuit inputs".into())
        })?;

    let client = Client::new();
    let new_root = bigint_to_be_bytes_array::<32>(&inputs.new_root).unwrap();
    let inputs = to_json(&inputs);

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
        Err(ForesterUtilsError::ProverError(
            "Prover failed to generate proof".to_string(),
        ))
    }
}

pub async fn create_append_batch_ix_data<R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    indexer: &mut I,
    merkle_tree_pubkey: Pubkey,
    output_queue_pubkey: Pubkey,
) -> Result<InstructionDataBatchAppendInputs, ForesterUtilsError> {
    let (merkle_tree_next_index, current_root) = {
        let mut merkle_tree_account = rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
        let merkle_tree =
            BatchedMerkleTreeAccount::state_from_bytes(merkle_tree_account.data.as_mut_slice())
                .unwrap();
        (
            merkle_tree.next_index,
            *merkle_tree.root_history.last().unwrap(),
        )
    };

    let (zkp_batch_size, full_batch_index, num_inserted_zkps, leaves_hashchain) = {
        let mut output_queue_account = rpc.get_account(output_queue_pubkey).await.unwrap().unwrap();
        let output_queue =
            BatchedQueueAccount::output_from_bytes(output_queue_account.data.as_mut_slice())
                .unwrap();

        let full_batch_index = output_queue.batch_metadata.next_full_batch_index;
        let zkp_batch_size = output_queue.batch_metadata.zkp_batch_size;

        let num_inserted_zkps =
            output_queue.batch_metadata.batches[full_batch_index as usize].get_num_inserted_zkps();

        let leaves_hashchain =
            output_queue.hash_chain_stores[full_batch_index as usize][num_inserted_zkps as usize];

        (
            zkp_batch_size,
            full_batch_index,
            num_inserted_zkps,
            leaves_hashchain,
        )
    };
    let start = num_inserted_zkps as usize * zkp_batch_size as usize;
    let end = start + zkp_batch_size as usize;

    let leaves = indexer
        .get_queue_elements(
            merkle_tree_pubkey.to_bytes(),
            full_batch_index,
            start as u64,
            end as u64,
        )
        .await
        .unwrap();

    info!("Leaves: {:?}", leaves);

    let (old_leaves, merkle_proofs) = {
        let mut old_leaves = vec![];
        let mut merkle_proofs = vec![];
        let indices =
            (merkle_tree_next_index..merkle_tree_next_index + zkp_batch_size).collect::<Vec<_>>();
        let proofs = indexer
            .get_proofs_by_indices(merkle_tree_pubkey, &indices)
            .await
            .unwrap();
        proofs.iter().for_each(|proof| {
            old_leaves.push(proof.leaf);
            merkle_proofs.push(proof.proof.clone());
        });

        (old_leaves, merkle_proofs)
    };

    info!("Old leaves: {:?}", old_leaves);

    let (proof, new_root) = {
        let circuit_inputs =
            get_batch_append_with_proofs_inputs::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                current_root,
                merkle_tree_next_index as u32,
                leaves,
                leaves_hashchain,
                old_leaves,
                merkle_proofs,
                zkp_batch_size as u32,
            )
            .unwrap();

        let client = Client::new();
        let inputs_json = BatchAppendWithProofsInputsJson::from_inputs(&circuit_inputs).to_string();

        let response = client
            .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(inputs_json)
            .send()
            .await
            .expect("Failed to execute request.");

        if response.status().is_success() {
            let body = response.text().await.unwrap();
            let proof_json = deserialize_gnark_proof_json(&body).unwrap();
            let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
            let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);
            (
                CompressedProof {
                    a: proof_a,
                    b: proof_b,
                    c: proof_c,
                },
                bigint_to_be_bytes_array::<32>(&circuit_inputs.new_root.to_biguint().unwrap())
                    .unwrap(),
            )
        } else {
            error!(
                "create_append_batch_ix_data: prover server respond: {:?}",
                response.text().await
            );
            return Err(ForesterUtilsError::ProverError(
                "Prover response failed".to_string(),
            ));
        }
    };

    Ok(InstructionDataBatchAppendInputs {
        new_root,
        compressed_proof: proof,
    })
}

pub async fn create_nullify_batch_ix_data<R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    indexer: &mut I,
    merkle_tree_pubkey: Pubkey,
) -> Result<InstructionDataBatchNullifyInputs, ForesterUtilsError> {
    let (zkp_batch_size, old_root, leaves_hashchain) = {
        let mut account = rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
        let merkle_tree =
            BatchedMerkleTreeAccount::state_from_bytes(account.data.as_mut_slice()).unwrap();
        let batch_idx = merkle_tree.queue_metadata.next_full_batch_index as usize;
        let zkp_size = merkle_tree.queue_metadata.zkp_batch_size;
        let batch = &merkle_tree.queue_metadata.batches[batch_idx];
        let zkp_idx = batch.get_num_inserted_zkps();
        let hashchain = merkle_tree.hash_chain_stores[batch_idx][zkp_idx as usize];
        let root = *merkle_tree.root_history.last().unwrap();
        (zkp_size, root, hashchain)
    };

    let leaf_indices_tx_hashes = indexer
        .get_leaf_indices_tx_hashes(merkle_tree_pubkey, zkp_batch_size as usize)
        .await
        .unwrap();

    let mut leaves = Vec::new();
    let mut tx_hashes = Vec::new();
    let mut old_leaves = Vec::new();
    let mut path_indices = Vec::new();
    let mut merkle_proofs = Vec::new();
    let mut nullifiers = Vec::new();

    let proofs = indexer
        .get_proofs_by_indices(
            merkle_tree_pubkey,
            &leaf_indices_tx_hashes
                .iter()
                .map(|leaf_info| leaf_info.leaf_index as u64)
                .collect::<Vec<_>>(),
        )
        .await
        .unwrap();

    for (leaf_info, proof) in leaf_indices_tx_hashes.iter().zip(proofs.iter()) {
        path_indices.push(leaf_info.leaf_index);
        leaves.push(leaf_info.leaf);
        old_leaves.push(proof.leaf);
        merkle_proofs.push(proof.proof.clone());
        tx_hashes.push(leaf_info.tx_hash);
        let index_bytes = leaf_info.leaf_index.to_be_bytes();
        let nullifier =
            Poseidon::hashv(&[&leaf_info.leaf, &index_bytes, &leaf_info.tx_hash]).unwrap();
        nullifiers.push(nullifier);
    }

    let inputs = get_batch_update_inputs::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
        old_root,
        tx_hashes,
        leaves.to_vec(),
        leaves_hashchain,
        old_leaves,
        merkle_proofs,
        path_indices,
        zkp_batch_size as u32,
    )
    .unwrap();

    let new_root = bigint_to_be_bytes_array::<32>(&inputs.new_root.to_biguint().unwrap()).unwrap();

    let client = Client::new();
    let response = client
        .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(update_inputs_string(&inputs))
        .send()
        .await
        .map_err(|e| {
            error!(
                "get_batched_nullify_ix_data: failed to send proof to server: {:?}",
                e
            );
            ForesterUtilsError::ProverError("Failed to send proof to server".into())
        })?;

    let proof = if response.status().is_success() {
        let body = response.text().await.unwrap();
        let proof_json = deserialize_gnark_proof_json(&body).unwrap();
        let (proof_a, proof_b, proof_c) = proof_from_json_struct(proof_json);
        let (proof_a, proof_b, proof_c) = compress_proof(&proof_a, &proof_b, &proof_c);
        CompressedProof {
            a: proof_a,
            b: proof_b,
            c: proof_c,
        }
    } else {
        error!(
            "get_batched_nullify_ix_data: failed to get proof from server: {:?}",
            response.text().await
        );
        return Err(ForesterUtilsError::ProverError(
            "Failed to get proof from server".into(),
        ));
    };

    Ok(InstructionDataBatchNullifyInputs {
        new_root,
        compressed_proof: proof,
    })
}
