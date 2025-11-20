use std::{collections::{HashMap, HashSet}, sync::Arc};

use solana_sdk::pubkey::Pubkey;
use tokio::sync::{Mutex as TokioMutex, RwLock};
use tracing::{debug, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcessedBatchId {
    pub batch_index: usize,
    pub zkp_batch_index: u64,
    pub is_append: bool,
    pub start_leaf_index: Option<u64>,
}

impl ProcessedBatchId {
    pub fn append(batch_index: usize, zkp_batch_index: u64) -> Self {
        Self {
            batch_index,
            zkp_batch_index,
            is_append: true,
            start_leaf_index: None,
        }
    }

    pub fn nullify(batch_index: usize, zkp_batch_index: u64) -> Self {
        Self {
            batch_index,
            zkp_batch_index,
            is_append: false,
            start_leaf_index: None,
        }
    }

    pub fn operation_type(&self) -> &'static str {
        if self.is_append {
            "append"
        } else {
            "nullify"
        }
    }
}

pub struct SharedTreeState {
    pub current_root: [u8; 32],
    pub processed_batches: HashSet<ProcessedBatchId>,
}

impl SharedTreeState {
    pub fn new(initial_root: [u8; 32]) -> Self {
        info!(
            "Initializing SharedTreeState with root: {:?}",
            &initial_root[..8]
        );
        Self {
            current_root: initial_root,
            processed_batches: HashSet::new(),
        }
    }

    pub fn update_root(&mut self, new_root: [u8; 32]) {
        self.current_root = new_root;
    }

    pub fn mark_batch_processed(&mut self, batch_id: ProcessedBatchId) {
        debug!(
            "Marking batch as processed: batch_idx={}, zkp_batch_idx={}, type={}, start_leaf={:?}",
            batch_id.batch_index,
            batch_id.zkp_batch_index,
            batch_id.operation_type(),
            batch_id.start_leaf_index
        );
        self.processed_batches.insert(batch_id);
    }

    pub fn is_batch_processed(&self, batch_id: &ProcessedBatchId) -> bool {
        self.processed_batches.contains(batch_id)
    }

    pub fn reset(
        &mut self,
        on_chain_root: [u8; 32],
        input_queue_batches: &[light_batched_merkle_tree::batch::Batch; 2],
        output_queue_batches: &[light_batched_merkle_tree::batch::Batch; 2],
    ) {
        let processed_count_before = self.processed_batches.len();
        let root_changed = self.current_root != on_chain_root;

        info!(
            "Resetting state: clearing {} processed batches, root_changed={}, syncing to root {:?}",
            processed_count_before,
            root_changed,
            &on_chain_root[..8]
        );

        self.current_root = on_chain_root;

        self.processed_batches.retain(|batch_id| {
            let on_chain_inserted = if batch_id.is_append {
                output_queue_batches
                    .get(batch_id.batch_index)
                    .map(|b| b.get_num_inserted_zkps())
                    .unwrap_or(0)
            } else {
                input_queue_batches
                    .get(batch_id.batch_index)
                    .map(|b| b.get_num_inserted_zkps())
                    .unwrap_or(0)
            };

            let should_keep = batch_id.zkp_batch_index > on_chain_inserted;

            if !should_keep {
                debug!(
                    "Clearing processed batch (confirmed on-chain or failed to insert): batch_idx={}, zkp_batch_idx={}, type={}, on_chain_inserted={}, start_leaf={:?}",
                    batch_id.batch_index,
                    batch_id.zkp_batch_index,
                    batch_id.operation_type(),
                    on_chain_inserted,
                    batch_id.start_leaf_index
                );
            }

            should_keep
        });

        let processed_count_after = self.processed_batches.len();
        let cleared_count = processed_count_before - processed_count_after;

        if cleared_count > 0 {
            info!(
                "Cleared {} processed batches that were confirmed on-chain, {} remaining",
                cleared_count, processed_count_after
            );
        } else if processed_count_after > 0 {
            debug!(
                "Keeping {} processed batches (not yet confirmed on-chain)",
                processed_count_after
            );
        }
    }

    /// Clears all processed batches (used at start of new epoch's active phase).
    pub fn clear_all_processed_batches(&mut self) {
        let count = self.processed_batches.len();
        if count > 0 {
            info!(
                "Clearing all {} processed batches at start of new epoch",
                count
            );
            self.processed_batches.clear();
        }
    }
}

pub type SharedState = Arc<RwLock<SharedTreeState>>;

pub fn create_shared_state(initial_root: [u8; 32]) -> SharedState {
    Arc::new(RwLock::new(SharedTreeState::new(initial_root)))
}

pub async fn get_or_create_shared_state(
    cache: &Arc<TokioMutex<HashMap<Pubkey, SharedState>>>,
    key: Pubkey,
    initial_root: [u8; 32],
) -> SharedState {
    let mut states = cache.lock().await;
    if let Some(state) = states.get(&key) {
        state.clone()
    } else {
        let new_state = create_shared_state(initial_root);
        states.insert(key, new_state.clone());
        new_state
    }
}
