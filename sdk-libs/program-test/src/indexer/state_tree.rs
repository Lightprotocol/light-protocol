use std::fmt::Debug;

use light_client::indexer::{IndexerError, StateMerkleTreeAccounts};
use light_compressed_account::TreeType;
use light_hasher::Poseidon;
use light_merkle_tree_reference::MerkleTree;

#[derive(Debug, Clone)]
pub struct LeafIndexInfo {
    pub leaf_index: u32,
    pub leaf: [u8; 32],
    pub tx_hash: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct StateMerkleTreeBundle {
    pub rollover_fee: i64,
    pub merkle_tree: Box<MerkleTree<Poseidon>>,
    pub accounts: StateMerkleTreeAccounts,
    pub tree_type: TreeType,
    pub output_queue_elements: Vec<([u8; 32], u64)>,
    pub input_leaf_indices: Vec<LeafIndexInfo>,
    pub output_queue_batch_size: Option<usize>,
    pub num_inserted_batches: usize,
}

impl StateMerkleTreeBundle {
    /// Returns true if index is in current queue range.
    pub fn leaf_index_in_queue_range(&self, index: usize) -> Result<bool, IndexerError> {
        if let Some(output_queue_batch_size) = self.output_queue_batch_size {
            let start_offset = self.num_inserted_batches * output_queue_batch_size;
            // There is always 2 batches.
            let end_offset = start_offset + (output_queue_batch_size * 2);
            Ok(start_offset <= index && index < end_offset)
        } else {
            Err(IndexerError::CustomError(format!(
                "Batch size not set for Merkle tree {:?}",
                self.accounts.merkle_tree
            )))
        }
    }
}
