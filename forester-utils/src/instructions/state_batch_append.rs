use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT,
    merkle_tree::{BatchedMerkleTreeAccount, InstructionDataBatchAppendInputs},
    queue::BatchedQueueAccount,
};
use light_client::{indexer::Indexer, rpc::RpcConnection};
use light_compressed_account::{
    bigint::bigint_to_be_bytes_array, instruction_data::compressed_proof::CompressedProof,
};
use light_merkle_tree_metadata::queue::QueueType;
use light_prover_client::{
    batch_append_with_proofs::get_batch_append_with_proofs_inputs,
    gnark::{
        batch_append_with_proofs_json_formatter::BatchAppendWithProofsInputsJson,
        constants::{PROVE_PATH, SERVER_ADDRESS},
        proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    },
};
use reqwest::Client;
use tracing::{debug, error};

use crate::{error::ForesterUtilsError, utils::wait_for_indexer};

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
    debug!(
        "merkle_tree_next_index: {:?} current_root: {:?}",
        merkle_tree_next_index, current_root
    );

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
    debug!(
        "zkp_batch_size: {:?} leaves_hash_chain: {:?}",
        zkp_batch_size, leaves_hash_chain
    );

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
            ForesterUtilsError::Indexer("Failed to get queue elements".into())
        })?;
    debug!("get_queue_elements len: {}", indexer_response.len());
    let indexer_root = indexer_response.first().unwrap().root;
    debug_assert_eq!(
        indexer_root, current_root,
        "root_history: {:?}",
        root_history
    );

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
                ForesterUtilsError::Prover("Failed to get circuit inputs".into())
            })?;
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
            return Err(ForesterUtilsError::Prover(
                "Prover response failed".to_string(),
            ));
        }
    };

    Ok(InstructionDataBatchAppendInputs {
        new_root,
        compressed_proof: proof,
    })
}
