use crate::processor::v2::coordinator::shared_state::ProcessedBatchId;
use light_batched_merkle_tree::batch::{Batch, BatchState};
use std::collections::HashSet;

/// Maximum number of times to resubmit a proof job when the prover returns "job_not_found".
/// This handles transient race conditions where the prover hasn't yet registered the job.
pub const MAX_JOB_NOT_FOUND_RESUBMITS: usize = 2;

pub fn count_ready_batches(
    batches: &[Batch; 2],
    processed_batches: &HashSet<ProcessedBatchId>,
    is_append: bool,
    calculate_start_index: bool,
) -> usize {
    let mut total_ready = 0;

    for (batch_idx, batch) in batches.iter().enumerate() {
        let batch_state = batch.get_state();
        if batch_state == BatchState::Inserted {
            continue;
        }

        let num_full_zkp_batches = batch.get_current_zkp_batch_index() as usize;
        let num_inserted_zkps = batch.get_num_inserted_zkps() as usize;

        for zkp_idx in num_inserted_zkps..num_full_zkp_batches {
            let start_leaf_index = if calculate_start_index {
                Some(batch.start_index + (zkp_idx as u64 * batch.zkp_batch_size))
            } else {
                None
            };

            let batch_id = ProcessedBatchId {
                batch_index: batch_idx,
                zkp_batch_index: zkp_idx as u64,
                is_append,
                start_leaf_index,
            };
            if !processed_batches.contains(&batch_id) {
                total_ready += 1;
            }
        }
    }

    total_ready
}