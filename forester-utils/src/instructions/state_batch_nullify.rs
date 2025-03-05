use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT,
    merkle_tree::{BatchedMerkleTreeAccount, InstructionDataBatchNullifyInputs},
    queue::BatchedQueueAccount,
};
use light_client::{indexer::Indexer, rpc::RpcConnection};
use light_compressed_account::{
    bigint::bigint_to_be_bytes_array, instruction_data::compressed_proof::CompressedProof,
};
use light_hasher::{Hasher, Poseidon};
use light_merkle_tree_metadata::queue::QueueType;
use light_prover_client::{
    batch_update::get_batch_update_inputs,
    gnark::{
        batch_update_json_formatter::update_inputs_string,
        constants::{PROVE_PATH, SERVER_ADDRESS},
        proof_helpers::{compress_proof, deserialize_gnark_proof_json, proof_from_json_struct},
    },
};
use reqwest::Client;
use tracing::{debug, error};

use crate::{error::ForesterUtilsError, utils::wait_for_indexer};

pub async fn create_nullify_batch_ix_data<R: RpcConnection, I: Indexer<R>>(
    rpc: &mut R,
    indexer: &mut I,
    merkle_tree_pubkey: Pubkey,
) -> Result<InstructionDataBatchNullifyInputs, ForesterUtilsError> {
    debug!("create_nullify_batch_ix_data");
    let (zkp_batch_size, old_root, root_history, leaves_hash_chain) = {
        let mut account = rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
        let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
            account.data.as_mut_slice(),
            &merkle_tree_pubkey.into(),
        )
        .unwrap();

        debug!("queue_batches: {:?}", merkle_tree.queue_batches);

        let batch_idx = merkle_tree.queue_batches.pending_batch_index as usize;
        let zkp_size = merkle_tree.queue_batches.zkp_batch_size;
        let batch = &merkle_tree.queue_batches.batches[batch_idx];
        let zkp_idx = batch.get_num_inserted_zkps();
        let hash_chain = merkle_tree.hash_chain_stores[batch_idx][zkp_idx as usize];
        let root = *merkle_tree.root_history.last().unwrap();
        let root_history = merkle_tree.root_history.to_vec();
        (zkp_size as u16, root, root_history, hash_chain)
    };
    debug!(
        "zkp_batch_size: {:?} old_root: {:?} : {:?}",
        zkp_batch_size, old_root, leaves_hash_chain
    );

    wait_for_indexer(rpc, indexer).await?;

    let current_slot = rpc.get_slot().await.unwrap();
    debug!("current_slot: {}", current_slot);

    let leaf_indices_tx_hashes = indexer
        .get_queue_elements(
            merkle_tree_pubkey.to_bytes(),
            QueueType::BatchedInput,
            zkp_batch_size,
            None,
        )
        .await
        .unwrap();

    debug!("get_queue_elements len: {}", leaf_indices_tx_hashes.len());

    let indexer_root = leaf_indices_tx_hashes.first().unwrap().root;

    debug_assert_eq!(indexer_root, old_root, "root_history: {:?}", root_history);

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
            ForesterUtilsError::Prover("Failed to send proof to server".into())
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

            let output_queue =
                BatchedQueueAccount::output_from_bytes(output_queue_account.data.as_mut_slice())
                    .unwrap();

            debug!("output queue metadata: {:?}", output_queue.get_metadata());
            debug!("tree metadata: {:?}", merkle_tree.get_metadata());
            debug!("root: {:?}", merkle_tree.get_root());
            for (i, root) in merkle_tree.root_history.iter().enumerate() {
                debug!("root {}: {:?}", i, root);
            }
        }

        return Err(ForesterUtilsError::Prover(
            "Failed to get proof from server".into(),
        ));
    };
    debug!("proof: {:?}", proof);

    Ok(InstructionDataBatchNullifyInputs {
        new_root,
        compressed_proof: proof,
    })
}
