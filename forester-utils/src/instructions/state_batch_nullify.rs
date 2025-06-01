use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT,
    merkle_tree::{BatchedMerkleTreeAccount, InstructionDataBatchNullifyInputs},
};
use light_client::{indexer::Indexer, rpc::RpcConnection};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_hasher::{bigint::bigint_to_be_bytes_array, Hasher, Poseidon};
use light_merkle_tree_metadata::QueueType;
use light_prover_client::{
    batch_update::{get_batch_update_inputs, BatchUpdateCircuitInputs},
    proof_client::ProofClient,
};
use tracing::{error, trace};

use crate::{error::ForesterUtilsError, utils::wait_for_indexer};

pub async fn create_nullify_batch_ix_data<R: RpcConnection, I: Indexer>(
    rpc: &mut R,
    indexer: &mut I,
    merkle_tree_pubkey: Pubkey,
) -> Result<Vec<InstructionDataBatchNullifyInputs>, ForesterUtilsError> {
    trace!("create_multiple_nullify_batch_ix_data");
    // Get the tree information and find out how many ZKP batches need processing
    let (
        batch_idx,
        zkp_batch_size,
        num_inserted_zkps,
        num_ready_zkps,
        old_root,
        root_history,
        leaves_hash_chains,
    ) = {
        let mut account = rpc.get_account(merkle_tree_pubkey).await.unwrap().unwrap();
        let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
            account.data.as_mut_slice(),
            &merkle_tree_pubkey.into(),
        )
        .unwrap();

        trace!("queue_batches: {:?}", merkle_tree.queue_batches);

        let batch_idx = merkle_tree.queue_batches.pending_batch_index as usize;
        let zkp_size = merkle_tree.queue_batches.zkp_batch_size;
        let batch = &merkle_tree.queue_batches.batches[batch_idx];
        let num_inserted_zkps = batch.get_num_inserted_zkps();
        let num_current_zkp = batch.get_current_zkp_batch_index();
        let num_ready_zkps = num_current_zkp.saturating_sub(num_inserted_zkps);

        let mut leaves_hash_chains = Vec::new();
        for i in num_inserted_zkps..num_current_zkp {
            leaves_hash_chains.push(merkle_tree.hash_chain_stores[batch_idx][i as usize]);
        }

        let root = *merkle_tree.root_history.last().unwrap();
        let root_history = merkle_tree.root_history.to_vec();

        (
            batch_idx,
            zkp_size as u16,
            num_inserted_zkps,
            num_ready_zkps,
            root,
            root_history,
            leaves_hash_chains,
        )
    };

    trace!(
        "batch_idx: {}, zkp_batch_size: {}, num_inserted_zkps: {}, num_ready_zkps: {}, leaves_hash_chains: {:?}",
        batch_idx, zkp_batch_size, num_inserted_zkps, num_ready_zkps, leaves_hash_chains.len()
    );

    if leaves_hash_chains.is_empty() {
        return Ok(Vec::new());
    }

    wait_for_indexer(rpc, indexer).await?;

    let current_slot = rpc.get_slot().await.unwrap();
    trace!("current_slot: {}", current_slot);

    let total_elements = zkp_batch_size as usize * leaves_hash_chains.len();
    let offset = num_inserted_zkps * zkp_batch_size as u64;

    trace!(
        "Requesting {} total elements with offset {}",
        total_elements,
        offset
    );

    let all_queue_elements = indexer
        .get_queue_elements(
            merkle_tree_pubkey.to_bytes(),
            QueueType::InputStateV2,
            total_elements as u16,
            Some(offset),
            None,
        )
        .await
        .map_err(|e| {
            error!(
                "create_multiple_nullify_batch_ix_data: failed to get queue elements from indexer: {:?}",
                e
            );
            ForesterUtilsError::Indexer("Failed to get queue elements".into())
        })?;

    trace!("Got {} queue elements in total", all_queue_elements.value.len());
    if all_queue_elements.value.len() != total_elements {
        return Err(ForesterUtilsError::Indexer(format!(
            "Expected {} elements, got {}",
            total_elements,
            all_queue_elements.value.len()
        )));
    }

    let indexer_root = all_queue_elements.value.first().unwrap().root;
    debug_assert_eq!(
        indexer_root, old_root,
        "Root mismatch. Expected: {:?}, Got: {:?}. Root history: {:?}",
        old_root, indexer_root, root_history
    );

    let mut all_changelogs = Vec::new();
    let mut proof_futures = Vec::new();

    let mut current_root = old_root;

    for (batch_offset, leaves_hash_chain) in leaves_hash_chains.iter().enumerate() {
        let start_idx = batch_offset * zkp_batch_size as usize;
        let end_idx = start_idx + zkp_batch_size as usize;
        let batch_elements = &all_queue_elements.value[start_idx..end_idx];

        trace!(
            "Processing batch {} with offset {}-{}",
            batch_offset,
            start_idx,
            end_idx
        );

        // Process this batch's data
        let mut leaves = Vec::new();
        let mut tx_hashes = Vec::new();
        let mut old_leaves = Vec::new();
        let mut path_indices = Vec::new();
        let mut merkle_proofs = Vec::new();
        let mut nullifiers = Vec::new();

        for (i, leaf_info) in batch_elements.iter().enumerate() {
            let global_leaf_index = start_idx + i;
            trace!(
                "Element {}: local index={}, global index={}, reported index={}",
                i,
                i,
                global_leaf_index,
                leaf_info.leaf_index
            );

            path_indices.push(leaf_info.leaf_index as u32);
            leaves.push(leaf_info.account_hash);
            old_leaves.push(leaf_info.leaf);
            merkle_proofs.push(leaf_info.proof.clone());

            // Make sure tx_hash exists
            let tx_hash = match leaf_info.tx_hash {
                Some(hash) => hash,
                None => {
                    return Err(ForesterUtilsError::Indexer(format!(
                        "Missing tx_hash for leaf index {}",
                        leaf_info.leaf_index
                    )))
                }
            };

            tx_hashes.push(tx_hash);

            let index_bytes = leaf_info.leaf_index.to_be_bytes();
            let nullifier =
                Poseidon::hashv(&[&leaf_info.account_hash, &index_bytes, &tx_hash]).unwrap();
            nullifiers.push(nullifier);
        }

        let (circuit_inputs, batch_changelog) =
            get_batch_update_inputs::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                current_root,
                tx_hashes.clone(),
                leaves.clone(),
                *leaves_hash_chain,
                old_leaves.clone(),
                merkle_proofs.clone(),
                path_indices.clone(),
                zkp_batch_size as u32,
                &all_changelogs,
            )
            .map_err(|e| {
                error!("Failed to get batch update inputs: {:?}", e);
                ForesterUtilsError::Prover("Failed to get batch update inputs".into())
            })?;

        all_changelogs.extend(batch_changelog);
        current_root =
            bigint_to_be_bytes_array::<32>(&circuit_inputs.new_root.to_biguint().unwrap())
                .map_err(|_| {
                    ForesterUtilsError::Prover("Failed to convert new root to bytes".into())
                })?;

        let proof_future = tokio::spawn(generate_nullify_zkp_proof(circuit_inputs));
        proof_futures.push(proof_future);
    }

    // Wait for all proof generation to complete
    let mut results = Vec::new();

    for (i, future) in futures::future::join_all(proof_futures)
        .await
        .into_iter()
        .enumerate()
    {
        match future {
            Ok(result) => match result {
                Ok((proof, new_root)) => {
                    results.push(InstructionDataBatchNullifyInputs {
                        new_root,
                        compressed_proof: proof,
                    });
                    trace!("Successfully generated proof for batch {}", i);
                }
                Err(e) => {
                    error!("Error generating proof for batch {}: {:?}", i, e);
                    return Err(e);
                }
            },
            Err(e) => {
                error!("Task error for batch {}: {:?}", i, e);
                return Err(ForesterUtilsError::Prover(format!(
                    "Task error for batch {}: {:?}",
                    i, e
                )));
            }
        }
    }

    Ok(results)
}
async fn generate_nullify_zkp_proof(
    inputs: BatchUpdateCircuitInputs,
) -> Result<(CompressedProof, [u8; 32]), ForesterUtilsError> {
    let proof_client = ProofClient::local();
    proof_client
        .generate_batch_update_proof(inputs)
        .await
        .map_err(|e| ForesterUtilsError::Prover(e.to_string()))
}
