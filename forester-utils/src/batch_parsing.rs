use light_batched_merkle_tree::{
    batch::BatchState, merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};

use crate::{error::ForesterUtilsError, ParsedMerkleTreeData, ParsedQueueData};

type Result<T> = std::result::Result<T, ForesterUtilsError>;

pub type BatchFilter = dyn Fn(usize, u64) -> bool + Send + Sync;

pub fn parse_merkle_tree_batch(
    merkle_tree: &BatchedMerkleTreeAccount,
) -> Result<(ParsedMerkleTreeData, bool)> {
    let batch_index = merkle_tree.queue_batches.pending_batch_index;
    let batch = merkle_tree
        .queue_batches
        .batches
        .get(batch_index as usize)
        .ok_or_else(|| ForesterUtilsError::Parse("Batch not found".to_string()))?;

    let num_inserted_zkps = batch.get_num_inserted_zkps();
    let current_zkp_batch_index = batch.get_current_zkp_batch_index();

    let mut leaves_hash_chains = Vec::new();
    for i in num_inserted_zkps..current_zkp_batch_index {
        leaves_hash_chains.push(merkle_tree.hash_chain_stores[batch_index as usize][i as usize]);
    }

    let parsed_data = ParsedMerkleTreeData {
        next_index: merkle_tree.next_index,
        current_root: *merkle_tree.root_history.last().unwrap(),
        root_history: merkle_tree.root_history.to_vec(),
        zkp_batch_size: batch.zkp_batch_size as u16,
        pending_batch_index: batch_index as u32,
        num_inserted_zkps,
        current_zkp_batch_index,
        batch_start_index: batch.start_index,
        leaves_hash_chains,
    };

    let is_ready =
        batch.get_state() != BatchState::Inserted && current_zkp_batch_index > num_inserted_zkps;

    Ok((parsed_data, is_ready))
}

pub fn parse_output_queue_batch(
    output_queue: &BatchedQueueAccount,
) -> Result<(ParsedQueueData, bool)> {
    let batch_index = output_queue.batch_metadata.pending_batch_index;
    let batch = output_queue
        .batch_metadata
        .batches
        .get(batch_index as usize)
        .ok_or_else(|| ForesterUtilsError::Parse("Batch not found".to_string()))?;

    let num_inserted_zkps = batch.get_num_inserted_zkps();
    let current_zkp_batch_index = batch.get_current_zkp_batch_index();

    let mut leaves_hash_chains = Vec::new();
    for i in num_inserted_zkps..current_zkp_batch_index {
        leaves_hash_chains.push(output_queue.hash_chain_stores[batch_index as usize][i as usize]);
    }

    let parsed_data = ParsedQueueData {
        zkp_batch_size: output_queue.batch_metadata.zkp_batch_size as u16,
        pending_batch_index: batch_index as u32,
        num_inserted_zkps,
        current_zkp_batch_index,
        leaves_hash_chains,
    };

    let is_ready =
        batch.get_state() != BatchState::Inserted && current_zkp_batch_index > num_inserted_zkps;

    Ok((parsed_data, is_ready))
}

/// The filter function receives (batch_index, zkp_batch_index) and returns true
/// if that batch should be included.
pub fn parse_merkle_tree_batches_filtered<F>(
    merkle_tree: &BatchedMerkleTreeAccount,
    filter: F,
) -> Result<ParsedMerkleTreeData>
where
    F: Fn(usize, u64) -> bool,
{
    let mut tree_leaves_hash_chains = Vec::new();
    let mut zkp_batch_size = 0u16;
    let mut batch_start_index = 0u64;

    for (batch_idx, batch) in merkle_tree.queue_batches.batches.iter().enumerate() {
        let batch_state = batch.get_state();
        let not_inserted = batch_state != BatchState::Inserted;

        if not_inserted {
            let num_inserted = batch.get_num_inserted_zkps();
            let current_index = batch.get_current_zkp_batch_index();

            if batch_idx == 0 || zkp_batch_size == 0 {
                zkp_batch_size = batch.zkp_batch_size as u16;
                batch_start_index = batch.start_index;
            }

            for i in num_inserted..current_index {
                if filter(batch_idx, i) {
                    tree_leaves_hash_chains
                        .push(merkle_tree.hash_chain_stores[batch_idx][i as usize]);
                }
            }
        }
    }

    Ok(ParsedMerkleTreeData {
        next_index: merkle_tree.next_index,
        current_root: *merkle_tree.root_history.last().unwrap(),
        root_history: merkle_tree.root_history.to_vec(),
        zkp_batch_size,
        pending_batch_index: merkle_tree.queue_batches.pending_batch_index as u32,
        num_inserted_zkps: 0,
        current_zkp_batch_index: 0,
        batch_start_index,
        leaves_hash_chains: tree_leaves_hash_chains,
    })
}

pub fn parse_output_queue_batches_filtered<F>(
    output_queue: &BatchedQueueAccount,
    filter: F,
) -> Result<ParsedQueueData>
where
    F: Fn(usize, u64) -> bool,
{
    let mut queue_leaves_hash_chains = Vec::new();

    for (batch_idx, batch) in output_queue.batch_metadata.batches.iter().enumerate() {
        let batch_state = batch.get_state();
        let not_inserted = batch_state != BatchState::Inserted;

        if not_inserted {
            let num_inserted = batch.get_num_inserted_zkps();
            let current_index = batch.get_current_zkp_batch_index();

            for i in num_inserted..current_index {
                if filter(batch_idx, i) {
                    queue_leaves_hash_chains
                        .push(output_queue.hash_chain_stores[batch_idx][i as usize]);
                }
            }
        }
    }

    Ok(ParsedQueueData {
        zkp_batch_size: output_queue.batch_metadata.zkp_batch_size as u16,
        pending_batch_index: output_queue.batch_metadata.pending_batch_index as u32,
        num_inserted_zkps: 0,
        current_zkp_batch_index: 0,
        leaves_hash_chains: queue_leaves_hash_chains,
    })
}
