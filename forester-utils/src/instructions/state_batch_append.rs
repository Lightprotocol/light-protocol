use std::{pin::Pin, sync::Arc, time::Duration};

use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use async_stream::stream;
use futures::{
    stream::{FuturesOrdered, Stream},
    StreamExt,
};
use light_batched_merkle_tree::{
    constants::DEFAULT_BATCH_STATE_TREE_HEIGHT, merkle_tree::InstructionDataBatchAppendInputs,
};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_account::instruction_data::compressed_proof::CompressedProof;
use light_hasher::bigint::bigint_to_be_bytes_array;
use light_merkle_tree_metadata::QueueType;
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::batch_append::{get_batch_append_inputs, BatchAppendsCircuitInputs},
};
use light_sparse_merkle_tree::changelog::ChangelogEntry;
use tracing::trace;

use crate::{
    error::ForesterUtilsError, rpc_pool::SolanaRpcPool, utils::wait_for_indexer,
    ParsedMerkleTreeData, ParsedQueueData,
};

async fn generate_zkp_proof(
    circuit_inputs: BatchAppendsCircuitInputs,
    proof_client: Arc<ProofClient>,
) -> Result<InstructionDataBatchAppendInputs, ForesterUtilsError> {
    let (proof, new_root) = proof_client
        .generate_batch_append_proof(circuit_inputs)
        .await
        .map_err(|e| ForesterUtilsError::Prover(e.to_string()))?;
    Ok(InstructionDataBatchAppendInputs {
        new_root,
        compressed_proof: CompressedProof {
            a: proof.a,
            b: proof.b,
            c: proof.c,
        },
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn get_append_instruction_stream<'a, R: Rpc>(
    rpc_pool: Arc<SolanaRpcPool<R>>,
    merkle_tree_pubkey: Pubkey,
    prover_url: String,
    polling_interval: Duration,
    max_wait_time: Duration,
    merkle_tree_data: ParsedMerkleTreeData,
    output_queue_data: ParsedQueueData,
    yield_batch_size: usize,
) -> Result<
    (
        Pin<
            Box<
                dyn Stream<Item = Result<Vec<InstructionDataBatchAppendInputs>, ForesterUtilsError>>
                    + Send
                    + 'a,
            >,
        >,
        u16,
    ),
    ForesterUtilsError,
> {
    trace!("Initializing append batch instruction stream with parsed data");
    let (merkle_tree_next_index, mut current_root, _) = (
        merkle_tree_data.next_index,
        merkle_tree_data.current_root,
        merkle_tree_data.root_history,
    );
    let (zkp_batch_size, leaves_hash_chains) = (
        output_queue_data.zkp_batch_size,
        output_queue_data.leaves_hash_chains,
    );

    if leaves_hash_chains.is_empty() {
        trace!("No hash chains to process, returning empty stream.");
        return Ok((Box::pin(futures::stream::empty()), zkp_batch_size));
    }
    let rpc = rpc_pool.get_connection().await?;
    wait_for_indexer(&*rpc).await?;
    drop(rpc);

    let stream = stream! {
        let total_elements = zkp_batch_size as usize * leaves_hash_chains.len();
        let offset = merkle_tree_next_index;

        let queue_elements = {
            let mut connection = rpc_pool.get_connection().await?;
            let indexer = connection.indexer_mut()?;
            match indexer
                .get_queue_elements(
                    merkle_tree_pubkey.to_bytes(),
                    QueueType::OutputStateV2,
                    total_elements as u16,
                    Some(offset),
                    None,
                )
                .await {
                    Ok(res) => res.value.items,
                    Err(e) => {
                        yield Err(ForesterUtilsError::Indexer(format!("Failed to get queue elements: {}", e)));
                        return;
                    }
                }
        };

        if queue_elements.len() != total_elements {
            yield Err(ForesterUtilsError::Indexer(format!(
                "Expected {} elements, got {}",
                total_elements,
                queue_elements.len()
            )));
            return;
        }

        if let Some(first_element) = queue_elements.first() {
            if first_element.root != current_root {
                 yield Err(ForesterUtilsError::Indexer("Root mismatch between indexer and on-chain state".into()));
                 return;
            }
        }

        let mut all_changelogs: Vec<ChangelogEntry<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>> = Vec::new();
        let proof_client = Arc::new(ProofClient::with_config(prover_url.clone(), polling_interval, max_wait_time));
        let mut futures_ordered = FuturesOrdered::new();
        let mut pending_count = 0;

        let mut proof_buffer = Vec::new();

        for (batch_idx, leaves_hash_chain) in leaves_hash_chains.iter().enumerate() {
            let start_idx = batch_idx * zkp_batch_size as usize;
            let end_idx = start_idx + zkp_batch_size as usize;
            let batch_elements = &queue_elements[start_idx..end_idx];

            let old_leaves: Vec<[u8; 32]> = batch_elements.iter().map(|x| x.leaf).collect();
            let leaves: Vec<[u8; 32]> = batch_elements.iter().map(|x| x.account_hash).collect();
            let merkle_proofs: Vec<Vec<[u8; 32]>> = batch_elements.iter().map(|x| x.proof.clone()).collect();
            let adjusted_start_index = merkle_tree_next_index as u32 + (batch_idx * zkp_batch_size as usize) as u32;

            let (circuit_inputs, batch_changelogs) = match get_batch_append_inputs::<32>(
                current_root, adjusted_start_index, leaves, *leaves_hash_chain, old_leaves, merkle_proofs, zkp_batch_size as u32, &all_changelogs,
            ) {
                Ok(inputs) => inputs,
                Err(e) => {
                    yield Err(ForesterUtilsError::Prover(format!("Failed to get circuit inputs: {}", e)));
                    return;
                }
            };

            current_root = bigint_to_be_bytes_array::<32>(&circuit_inputs.new_root.to_biguint().unwrap()).unwrap();
            all_changelogs.extend(batch_changelogs);

            let client = Arc::clone(&proof_client);
            futures_ordered.push_back(generate_zkp_proof(circuit_inputs, client));
            pending_count += 1;

            while pending_count >= yield_batch_size {
                for _ in 0..yield_batch_size.min(pending_count) {
                    if let Some(result) = futures_ordered.next().await {
                        match result {
                            Ok(proof) => proof_buffer.push(proof),
                            Err(e) => {
                                yield Err(e);
                                return;
                            }
                        }
                        pending_count -= 1;
                    }
                }

                if !proof_buffer.is_empty() {
                    yield Ok(proof_buffer.clone());
                    proof_buffer.clear();
                }
            }
        }

        while let Some(result) = futures_ordered.next().await {
            match result {
                Ok(proof) => {
                    proof_buffer.push(proof);

                    if proof_buffer.len() >= yield_batch_size {
                        yield Ok(proof_buffer.clone());
                        proof_buffer.clear();
                    }
                },
                Err(e) => {
                    yield Err(e);
                    return;
                }
            }
        }

        // Yield any remaining proofs
        if !proof_buffer.is_empty() {
            yield Ok(proof_buffer);
        }
    };

    Ok((Box::pin(stream), zkp_batch_size))
}
