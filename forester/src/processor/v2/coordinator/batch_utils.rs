use crate::processor::v2::coordinator::error::CoordinatorError;
use crate::processor::v2::coordinator::shared_state::ProcessedBatchId;
use anyhow::Result;
use light_batched_merkle_tree::batch::{Batch, BatchState};
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use std::collections::HashSet;
use tracing::{info, warn};

/// Maximum number of times to resubmit a proof job when the prover returns "job_not_found".
/// This handles transient race conditions where the prover hasn't yet registered the job.
pub const MAX_JOB_NOT_FOUND_RESUBMITS: usize = 2;

/// Maximum number of consecutive retries for coordinator operations.
pub const MAX_COORDINATOR_RETRIES: usize = 10;

/// Maximum retries when Photon indexer data is stale.
pub const PHOTON_STALE_MAX_RETRIES: usize = 5;

/// Delay between retries when Photon data is stale (milliseconds).
pub const PHOTON_STALE_RETRY_DELAY_MS: u64 = 1500;

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

/// Validates that the current on-chain root matches the expected root.
/// Returns an error if roots don't match (indicates multi-forester race condition).
pub fn validate_root(current_root: [u8; 32], expected_root: [u8; 32], phase: &str) -> Result<()> {
    if current_root != expected_root {
        let mut expected = [0u8; 8];
        let mut actual = [0u8; 8];
        expected.copy_from_slice(&expected_root[..8]);
        actual.copy_from_slice(&current_root[..8]);

        warn!(
            "Root changed during {} (multi-forester race): expected {:?}, now {:?}",
            phase, expected, actual
        );

        return Err(CoordinatorError::RootChanged {
            phase: phase.to_string(),
            expected,
            actual,
        }
        .into());
    }

    info!("Root validation passed: {:?}", &expected_root[..8]);
    Ok(())
}

/// Validates that Photon indexer root matches the current on-chain root.
/// Returns an error if roots don't match (indicates stale indexer data).
pub fn validate_photon_root(
    photon_root: [u8; 32],
    onchain_root: [u8; 32],
    queue_type: &str,
) -> Result<()> {
    if photon_root != onchain_root {
        let mut photon = [0u8; 8];
        let mut onchain = [0u8; 8];
        photon.copy_from_slice(&photon_root[..8]);
        onchain.copy_from_slice(&onchain_root[..8]);

        return Err(CoordinatorError::PhotonStale {
            queue_type: queue_type.to_string(),
            photon_root: photon,
            onchain_root: onchain,
        }
        .into());
    }
    Ok(())
}

/// Extracts the current root from a batched merkle tree account.
/// Returns an error if the root history is empty.
pub fn extract_current_root(tree_data: &BatchedMerkleTreeAccount) -> Result<[u8; 32]> {
    tree_data
        .root_history
        .last()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("No root in tree history"))
}