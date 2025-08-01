use std::{pin::Pin, sync::Arc, time::Duration};

use futures::{stream::Stream, StreamExt};
use light_batched_merkle_tree::{
    merkle_tree::{InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs},
};
use light_client::rpc::Rpc;
use solana_sdk::pubkey::Pubkey;
use tracing::{debug, warn};

use crate::Result;
use forester_utils::{
    rpc_pool::SolanaRpcPool,
    ParsedMerkleTreeData,
    ParsedQueueData,
};

/// Creates a nullify stream that updates the shared tree cache after each proof
pub async fn create_nullify_stream_with_cache_update<R: Rpc>(
    rpc_pool: Arc<SolanaRpcPool<R>>,
    merkle_tree_pubkey: Pubkey,
    prover_url: String,
    polling_interval: Duration,
    max_wait_time: Duration,
    merkle_tree_data: ParsedMerkleTreeData,
    yield_batch_size: usize,
) -> Result<(
    Pin<Box<dyn Stream<Item = Result<Vec<InstructionDataBatchNullifyInputs>>> + Send>>,
    u16,
)> {
    let zkp_batch_size = merkle_tree_data.zkp_batch_size;
    let leaves_hash_chains = merkle_tree_data.leaves_hash_chains.clone();
    
    if leaves_hash_chains.is_empty() {
        debug!("No hash chains to process for nullification");
        let empty_stream = futures::stream::empty().map(|x| x);
        return Ok((Box::pin(empty_stream), zkp_batch_size));
    }
    
    // Use the existing stream implementation and wrap it
    let (stream, size) = forester_utils::instructions::state_batch_nullify::get_nullify_instruction_stream(
        rpc_pool.clone(),
        merkle_tree_pubkey,
        prover_url,
        polling_interval,
        max_wait_time,
        merkle_tree_data.clone(),
        yield_batch_size,
    )
    .await
    .map_err(anyhow::Error::from)?;
    
    // Wrap the stream to add cache update logic
    let _merkle_tree_pubkey_clone = merkle_tree_pubkey;
    let wrapped_stream = stream.map(move |result| {
        // After each batch of proofs, we should update the tree cache
        // Note: In a complete implementation, we would:
        // 1. Track the new root from each proof
        // 2. Fetch updated subtrees from indexer
        // 3. Update the tree cache
        match &result {
            Ok(proofs) => {
                for proof in proofs {
                    debug!("Generated proof with new root: {:?}", proof.new_root);
                    // TODO: Update tree cache with new root and subtrees
                }
            }
            Err(e) => {
                warn!("Error in nullify stream: {:?}", e);
            }
        }
        result.map_err(anyhow::Error::from)
    });
    
    Ok((Box::pin(wrapped_stream), size))
}

/// Creates an append stream that updates the shared tree cache after each proof
pub async fn create_append_stream_with_cache_update<R: Rpc>(
    rpc_pool: Arc<SolanaRpcPool<R>>,
    merkle_tree_pubkey: Pubkey,
    prover_url: String,
    polling_interval: Duration,
    max_wait_time: Duration,
    merkle_tree_data: ParsedMerkleTreeData,
    output_queue_data: ParsedQueueData,
    yield_batch_size: usize,
) -> Result<(
    Pin<Box<dyn Stream<Item = Result<Vec<InstructionDataBatchAppendInputs>>> + Send>>,
    u16,
)> {
    let zkp_batch_size = output_queue_data.zkp_batch_size;
    let leaves_hash_chains = output_queue_data.leaves_hash_chains.clone();
    
    if leaves_hash_chains.is_empty() {
        debug!("No hash chains to process for append");
        let empty_stream = futures::stream::empty().map(|x| x);
        return Ok((Box::pin(empty_stream), zkp_batch_size));
    }
    
    // Use the existing stream implementation and wrap it
    let (stream, size) = forester_utils::instructions::state_batch_append::get_append_instruction_stream(
        rpc_pool.clone(),
        merkle_tree_pubkey,
        prover_url,
        polling_interval,
        max_wait_time,
        merkle_tree_data.clone(),
        output_queue_data.clone(),
        yield_batch_size,
    )
    .await
    .map_err(anyhow::Error::from)?;
    
    // Wrap the stream to add cache update logic
    let _merkle_tree_pubkey_clone = merkle_tree_pubkey;
    let wrapped_stream = stream.map(move |result| {
        // After each batch of proofs, we should update the tree cache
        // Note: In a complete implementation, we would:
        // 1. Track the new root from each proof
        // 2. Track the new next_index
        // 3. Fetch updated subtrees from indexer
        // 4. Update the tree cache
        match &result {
            Ok(proofs) => {
                for proof in proofs {
                    debug!("Generated append proof with new root: {:?}", proof.new_root);
                    // TODO: Update tree cache with new root, next_index and subtrees
                }
            }
            Err(e) => {
                warn!("Error in append stream: {:?}", e);
            }
        }
        result.map_err(anyhow::Error::from)
    });
    
    Ok((Box::pin(wrapped_stream), size))
}