use light_batched_merkle_tree::{
    constants::{DEFAULT_BATCH_ADDRESS_TREE_HEIGHT, DEFAULT_BATCH_STATE_TREE_HEIGHT},
    merkle_tree::{
        BatchedMerkleTreeAccount, InstructionDataBatchAppendInputs,
        InstructionDataBatchNullifyInputs,
    },
    queue::BatchedQueueAccount,
};
use light_client::{indexer::Indexer, rpc::RpcConnection};
use light_compressed_account::{
    bigint::bigint_to_be_bytes_array, instruction_data::compressed_proof::CompressedProof,
};
use light_hasher::{Hasher, Poseidon};
use light_merkle_tree_metadata::queue::QueueType;
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
use log::error;
use reqwest::Client;
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;
use tokio::time::sleep;

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

    let (leaves_hash_chain, start_index, current_root, batch_size) = {
        let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &merkle_tree_pubkey.into(),
        )
        .unwrap();

        let full_batch_index = merkle_tree.queue_batches.pending_batch_index;
        let batch = &merkle_tree.queue_batches.batches[full_batch_index as usize];
        let zkp_batch_index = batch.get_num_inserted_zkps();
        let leaves_hash_chain =
            merkle_tree.hash_chain_stores[full_batch_index as usize][zkp_batch_index as usize];
        let start_index = merkle_tree.next_index;
        let current_root = *merkle_tree.root_history.last().unwrap();
        let zkp_batch_size = batch.zkp_batch_size as u16;

        (leaves_hash_chain, start_index, current_root, zkp_batch_size)
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
            QueueType::BatchedAddress,
            batch_size,
            None
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
            addresses.iter().map(|x|x.account_hash).collect(),
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
    let addresses = addresses
        .iter()
        .map(|x| x.account_hash)
        .collect::<Vec<[u8; 32]>>();

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
            leaves_hash_chain,
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
    println!("create_append_batch_ix_data");
    let (merkle_tree_next_index, current_root, root_history) = {
        let mut merkle_tree_account = rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
        let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &merkle_tree_pubkey.into(),
        )
        .unwrap();

        (
            merkle_tree.next_index,
            *merkle_tree.root_history.last().unwrap(),
            merkle_tree.root_history.to_vec(),
        )
    };
    println!("merkle_tree_next_index: {:?} current_root: {:?}", merkle_tree_next_index, current_root);

    let (zkp_batch_size, leaves_hash_chain) = {
        let mut output_queue_account = rpc.get_account(output_queue_pubkey).await.unwrap().unwrap();
        let output_queue =
            BatchedQueueAccount::output_from_bytes(output_queue_account.data.as_mut_slice())
                .unwrap();

        let full_batch_index = output_queue.batch_metadata.pending_batch_index;
        let zkp_batch_size = output_queue.batch_metadata.zkp_batch_size;

        let num_inserted_zkps =
            output_queue.batch_metadata.batches[full_batch_index as usize].get_num_inserted_zkps();

        let leaves_hash_chain =
            output_queue.hash_chain_stores[full_batch_index as usize][num_inserted_zkps as usize];
        (zkp_batch_size as u16, leaves_hash_chain)
    };
    println!("zkp_batch_size: {:?} leaves_hash_chain: {:?}", zkp_batch_size, leaves_hash_chain);

    wait_for_indexer(rpc, indexer).await?;

    let indexer_response = indexer
        .get_queue_elements(
            merkle_tree_pubkey.to_bytes(),
            QueueType::BatchedOutput,
            zkp_batch_size,
            None,
        )
        .await
        .map_err(|e| {
            error!(
                "create_append_batch_ix_data: failed to get queue elements from indexer: {:?}",
                e
            );
            ForesterUtilsError::IndexerError("Failed to get queue elements".into())
        })?;
    println!("get_queue_elements len: {}", indexer_response.len());
    let indexer_root = indexer_response.first().unwrap().root;
    assert_eq!(indexer_root, current_root, "root_history: {:?}", root_history);

    let old_leaves = indexer_response
        .iter()
        .map(|x| x.leaf)
        .collect::<Vec<[u8; 32]>>();
    let leaves = indexer_response
        .iter()
        .map(|x| x.account_hash)
        .collect::<Vec<[u8; 32]>>();
    let merkle_proofs = indexer_response
        .iter()
        .map(|x| x.proof.clone())
        .collect::<Vec<Vec<[u8; 32]>>>();

    let (proof, new_root) = {
        let circuit_inputs =
            get_batch_append_with_proofs_inputs::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                current_root,
                merkle_tree_next_index as u32,
                leaves,
                leaves_hash_chain,
                old_leaves,
                merkle_proofs,
                zkp_batch_size as u32,
            )
                .map_err(|e| {
                    error!(
                        "create_append_batch_ix_data: failed to get circuit inputs: {:?}",
                        e
                    );
                    ForesterUtilsError::ProverError("Failed to get circuit inputs".into())
                })?;
        let client = Client::new();
        let inputs_json = BatchAppendWithProofsInputsJson::from_inputs(&circuit_inputs).to_string();

        println!("inputs_json: {:?}", inputs_json);
        let response = client
            .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(inputs_json)
            .send()
            .await
            .expect("Failed to execute request.");
        println!("response: {:?}", response);
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
    println!("create_nullify_batch_ix_data");
    let (zkp_batch_size, old_root, root_history, leaves_hash_chain) = {
        let mut account = rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
        let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
            account.data.as_mut_slice(),
            &merkle_tree_pubkey.into(),
        )
        .unwrap();

        println!("queue_batches: {:?}", merkle_tree.queue_batches);

        let batch_idx = merkle_tree.queue_batches.pending_batch_index as usize;
        let zkp_size = merkle_tree.queue_batches.zkp_batch_size;
        let batch = &merkle_tree.queue_batches.batches[batch_idx];
        let zkp_idx = batch.get_num_inserted_zkps();
        let hash_chain = merkle_tree.hash_chain_stores[batch_idx][zkp_idx as usize];
        let root = *merkle_tree.root_history.last().unwrap();
        let root_history = merkle_tree.root_history.to_vec();
        (zkp_size as u16, root, root_history, hash_chain)
    };
    println!("zkp_batch_size: {:?} old_root: {:?} : {:?}", zkp_batch_size, old_root, leaves_hash_chain);

    wait_for_indexer(rpc, indexer).await?;

    let current_slot = rpc.get_slot().await.unwrap();
    println!("current_slot: {}", current_slot);

    let leaf_indices_tx_hashes = indexer
        .get_queue_elements(
            merkle_tree_pubkey.to_bytes(),
            QueueType::BatchedInput,
            zkp_batch_size,
            None,
        )
        .await
        .unwrap();

    println!("get_queue_elements len: {}", leaf_indices_tx_hashes.len());

    let indexer_root = leaf_indices_tx_hashes.first().unwrap().root;

    assert_eq!(indexer_root, old_root, "root_history: {:?}", root_history);

    let mut leaves = Vec::new();
    let mut tx_hashes = Vec::new();
    let mut old_leaves = Vec::new();
    let mut path_indices = Vec::new();
    let mut merkle_proofs = Vec::new();
    let mut nullifiers = Vec::new();

    for leaf_info in leaf_indices_tx_hashes.iter() {
        path_indices.push(leaf_info.leaf_index as u32);
        leaves.push(leaf_info.account_hash);
        old_leaves.push(leaf_info.leaf);
        merkle_proofs.push(leaf_info.proof.clone());
        tx_hashes.push(leaf_info.tx_hash.unwrap());
        let index_bytes = leaf_info.leaf_index.to_be_bytes();
        let nullifier = Poseidon::hashv(&[
            &leaf_info.account_hash,
            &index_bytes,
            &leaf_info.tx_hash.unwrap(),
        ])
        .unwrap();
        nullifiers.push(nullifier);
    }

    let inputs = get_batch_update_inputs::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
        old_root,
        tx_hashes,
        leaves.to_vec(),
        leaves_hash_chain,
        old_leaves,
        merkle_proofs,
        path_indices,
        zkp_batch_size as u32,
    )
    .unwrap();

    let new_root = bigint_to_be_bytes_array::<32>(&inputs.new_root.to_biguint().unwrap()).unwrap();

    let client = Client::new();

    let json_str = update_inputs_string(&inputs);
    let response = client
        .post(format!("{}{}", SERVER_ADDRESS, PROVE_PATH))
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(json_str.clone())
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
        println!(
            "get_batched_nullify_ix_data: failed to get proof from server: {:?}, input: {:?}",
            response.text().await,
            json_str
        );
        {
            let mut account = rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
            let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
                account.data.as_mut_slice(),
                &merkle_tree_pubkey.into(),
            )
                .unwrap();
            let batched_output_queue = merkle_tree.metadata.associated_queue;
            let mut output_queue_account = rpc
                .get_account(Pubkey::from(batched_output_queue))
                .await
                .unwrap()
                .unwrap();

            let output_queue = BatchedQueueAccount::output_from_bytes(
                output_queue_account.data.as_mut_slice(),
            )
                .unwrap();

            println!("output queue metadata: {:?}", output_queue.get_metadata());
            println!("tree metadata: {:?}", merkle_tree.get_metadata());
            println!("root: {:?}", merkle_tree.get_root());
            for (i, root) in merkle_tree.root_history.iter().enumerate() {
                println!("root {}: {:?}", i, root);
            }
        }

        return Err(ForesterUtilsError::ProverError(
            "Failed to get proof from server".into(),
        ));
    };
    println!("proof: {:?}", proof);

    Ok(InstructionDataBatchNullifyInputs {
        new_root,
        compressed_proof: proof,
    })
}

async fn wait_for_indexer<R: RpcConnection, I: Indexer<R>>(rpc: &mut R, indexer: &mut I) -> Result<(), ForesterUtilsError> {
    let rpc_slot = rpc.get_slot().await.map_err(|e| {
        error!(
            "create_nullify_batch_ix_data: failed to get rpc slot from rpc: {:?}",
            e
        );
        ForesterUtilsError::RpcError("Failed to get rpc slot".into())
    })?;

    let mut indexer_slot = indexer.get_indexer_slot().await.map_err(|e| {
        error!(
            "create_nullify_batch_ix_data: failed to get indexer slot from indexer: {:?}",
            e
        );
        ForesterUtilsError::IndexerError("Failed to get indexer slot".into())
    })?;

    while rpc_slot > indexer_slot {
        println!("waiting for indexer to catch up, rpc_slot: {}, indexer_slot: {}", rpc_slot, indexer_slot);
        sleep(std::time::Duration::from_millis(50)).await;
        indexer_slot = indexer.get_indexer_slot().await.map_err(|e| {
            error!(
            "create_nullify_batch_ix_data: failed to get indexer slot from indexer: {:?}",
            e
        );
            ForesterUtilsError::IndexerError("Failed to get indexer slot".into())
        })?;
    }
    Ok(())
}
