use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT,
    merkle_tree::{BatchedMerkleTreeAccount, InstructionDataBatchAppendInputs},
    queue::BatchedQueueAccount,
};
use light_client::{indexer::Indexer, rpc::RpcConnection};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_concurrent_merkle_tree::changelog::ChangelogEntry;
use light_hasher::bigint::bigint_to_be_bytes_array;
use light_merkle_tree_metadata::QueueType;
use light_prover_client::{
    batch_append_with_proofs::{
        get_batch_append_with_proofs_inputs, BatchAppendWithProofsCircuitInputs,
    },
    proof_client::ProofClient,
};
use tracing::{error, trace};

use crate::{error::ForesterUtilsError, utils::wait_for_indexer};

pub async fn create_append_batch_ix_data<R: RpcConnection, I: Indexer>(
    rpc: &mut R,
    indexer: &mut I,
    merkle_tree_pubkey: Pubkey,
    output_queue_pubkey: Pubkey,
) -> Result<Vec<InstructionDataBatchAppendInputs>, ForesterUtilsError> {
    trace!("Creating append batch instruction data");

    let (merkle_tree_next_index, current_root, root_history) =
        get_merkle_tree_metadata(rpc, merkle_tree_pubkey).await?;

    trace!(
        "merkle_tree_next_index: {:?} current_root: {:?}",
        merkle_tree_next_index,
        current_root
    );

    // Get output queue metadata and hash chains
    let (zkp_batch_size, leaves_hash_chains) =
        get_output_queue_metadata(rpc, output_queue_pubkey).await?;

    if leaves_hash_chains.is_empty() {
        trace!("No hash chains to process");
        return Ok(Vec::new());
    }

    wait_for_indexer(rpc, indexer).await?;

    let total_elements = zkp_batch_size as usize * leaves_hash_chains.len();
    let offset = merkle_tree_next_index;

    let queue_elements = indexer
        .get_queue_elements(
            merkle_tree_pubkey.to_bytes(),
            QueueType::OutputStateV2,
            total_elements as u16,
            Some(offset),
            None,
        )
        .await
        .map_err(|e| {
            error!("Failed to get queue elements from indexer: {:?}", e);
            ForesterUtilsError::Indexer("Failed to get queue elements".into())
        })?
        .value
        .items;

    trace!("Got {} queue elements in total", queue_elements.len());

    if queue_elements.len() != total_elements {
        return Err(ForesterUtilsError::Indexer(format!(
            "Expected {} elements, got {}",
            total_elements,
            queue_elements.len()
        )));
    }
    let indexer_root = queue_elements.first().unwrap().root;
    debug_assert_eq!(
        indexer_root, current_root,
        "root_history: {:?}",
        root_history
    );

    let mut current_root = current_root;
    let mut all_changelogs: Vec<ChangelogEntry<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>> =
        Vec::new();
    let mut proof_futures = Vec::new();

    for (batch_idx, leaves_hash_chain) in leaves_hash_chains.iter().enumerate() {
        let start_idx = batch_idx * zkp_batch_size as usize;
        let end_idx = start_idx + zkp_batch_size as usize;
        let batch_elements = &queue_elements[start_idx..end_idx];

        trace!(
            "Processing batch {}: index range {}-{}",
            batch_idx,
            start_idx,
            end_idx
        );

        let old_leaves = batch_elements
            .iter()
            .map(|x| x.leaf)
            .collect::<Vec<[u8; 32]>>();

        let leaves = batch_elements
            .iter()
            .map(|x| x.account_hash)
            .collect::<Vec<[u8; 32]>>();

        let merkle_proofs = batch_elements
            .iter()
            .map(|x| x.proof.clone())
            .collect::<Vec<Vec<[u8; 32]>>>();

        let adjusted_start_index =
            merkle_tree_next_index as u32 + (batch_idx * zkp_batch_size as usize) as u32;

        let (circuit_inputs, batch_changelogs) = get_batch_append_with_proofs_inputs(
            current_root,
            adjusted_start_index,
            leaves,
            *leaves_hash_chain,
            old_leaves,
            merkle_proofs,
            zkp_batch_size as u32,
            &all_changelogs,
        )
        .map_err(|e| {
            error!("Failed to get circuit inputs: {:?}", e);
            ForesterUtilsError::Prover("Failed to get circuit inputs".into())
        })?;

        current_root =
            bigint_to_be_bytes_array::<32>(&circuit_inputs.new_root.to_biguint().unwrap()).unwrap();
        all_changelogs.extend(batch_changelogs);

        let proof_future = generate_zkp_proof(circuit_inputs);

        proof_futures.push(proof_future);
    }

    let proof_results = futures::future::join_all(proof_futures).await;
    let mut instruction_data_vec = Vec::new();

    for (i, proof_result) in proof_results.into_iter().enumerate() {
        match proof_result {
            Ok((proof, new_root)) => {
                trace!("Successfully generated proof for batch {}", i);
                instruction_data_vec.push(InstructionDataBatchAppendInputs {
                    new_root,
                    compressed_proof: proof,
                });
            }
            Err(e) => {
                error!("Failed to generate proof for batch {}: {:?}", i, e);
                return Err(e);
            }
        }
    }

    Ok(instruction_data_vec)
}
async fn generate_zkp_proof(
    circuit_inputs: BatchAppendWithProofsCircuitInputs,
) -> Result<(CompressedProof, [u8; 32]), ForesterUtilsError> {
    let proof_client = ProofClient::local();
    proof_client
        .generate_batch_append_proof(circuit_inputs)
        .await
        .map_err(|e| ForesterUtilsError::Prover(e.to_string()))
}

/// Get metadata from the Merkle tree account
async fn get_merkle_tree_metadata(
    rpc: &mut impl RpcConnection,
    merkle_tree_pubkey: Pubkey,
) -> Result<(u64, [u8; 32], Vec<[u8; 32]>), ForesterUtilsError> {
    let mut merkle_tree_account = rpc
        .get_account(merkle_tree_pubkey)
        .await
        .map_err(|e| ForesterUtilsError::Rpc(format!("Failed to get merkle tree account: {}", e)))?
        .ok_or_else(|| ForesterUtilsError::Rpc("Merkle tree account not found".into()))?;

    let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
        merkle_tree_account.data.as_mut_slice(),
        &merkle_tree_pubkey.into(),
    )
    .map_err(|e| ForesterUtilsError::Rpc(format!("Failed to parse merkle tree: {}", e)))?;

    Ok((
        merkle_tree.next_index,
        *merkle_tree.root_history.last().unwrap(),
        merkle_tree.root_history.to_vec(),
    ))
}

/// Get metadata and hash chains from the output queue
async fn get_output_queue_metadata(
    rpc: &mut impl RpcConnection,
    output_queue_pubkey: Pubkey,
) -> Result<(u16, Vec<[u8; 32]>), ForesterUtilsError> {
    let mut output_queue_account = rpc
        .get_account(output_queue_pubkey)
        .await
        .map_err(|e| ForesterUtilsError::Rpc(format!("Failed to get output queue account: {}", e)))?
        .ok_or_else(|| ForesterUtilsError::Rpc("Output queue account not found".into()))?;

    let output_queue =
        BatchedQueueAccount::output_from_bytes(output_queue_account.data.as_mut_slice())
            .map_err(|e| ForesterUtilsError::Rpc(format!("Failed to parse output queue: {}", e)))?;

    let full_batch_index = output_queue.batch_metadata.pending_batch_index;
    let zkp_batch_size = output_queue.batch_metadata.zkp_batch_size;
    let batch = &output_queue.batch_metadata.batches[full_batch_index as usize];
    let num_inserted_zkps = batch.get_num_inserted_zkps();

    // Get all remaining hash chains for the batch
    let mut leaves_hash_chains = Vec::new();
    for i in num_inserted_zkps..batch.get_current_zkp_batch_index() {
        leaves_hash_chains
            .push(output_queue.hash_chain_stores[full_batch_index as usize][i as usize]);
    }

    trace!(
        "ZKP batch size: {}, inserted ZKPs: {}, current ZKP index: {}, ready for insertion: {}",
        zkp_batch_size,
        num_inserted_zkps,
        batch.get_current_zkp_batch_index(),
        leaves_hash_chains.len()
    );

    Ok((zkp_batch_size as u16, leaves_hash_chains))
}
