/// Generic proof polling pipeline with retry logic.
///
/// This module provides utilities for submitting proof jobs to a prover service
/// and polling for completion with automatic retry handling for transient errors.
use anyhow::Result;
use light_prover_client::proof_client::ProofClient;
use std::sync::Arc;
use tracing::{info, warn};

use super::batch_utils::MAX_JOB_NOT_FOUND_RESUBMITS;

/// Polls for proof completion with automatic retry on job_not_found errors.
///
/// This function implements the retry logic used across all proof types:
/// 1. Poll for proof completion
/// 2. If job_not_found error occurs, resubmit the job (up to MAX_JOB_NOT_FOUND_RESUBMITS times)
/// 3. Return proof on success or error after max retries
///
/// # Arguments
/// * `client` - Proof client to use for polling and resubmission
/// * `initial_job_id` - Initial job ID to poll
/// * `inputs_json` - JSON string of circuit inputs (for resubmission)
/// * `proof_type` - Type string for the proof ("append", "update", "address")
/// * `batch_idx` - Batch index (for logging)
/// * `on_success` - Callback to convert successful proof into result type T
///
/// # Returns
/// Result containing either the converted proof (type T) or an error
pub async fn poll_proof_with_retry<T, F>(
    client: Arc<ProofClient>,
    initial_job_id: String,
    inputs_json: String,
    proof_type: &str,
    batch_idx: usize,
    on_success: F,
) -> Result<T>
where
    F: FnOnce(light_prover_client::proof::ProofCompressed) -> Result<T>,
{
    let mut current_job = initial_job_id;
    let mut resubmits = 0usize;

    loop {
        let result = client.poll_proof_completion(current_job.clone()).await;

        match result {
            Ok(proof) => {
                // Success - convert proof and return
                return on_success(proof);
            }
            Err(e) if e.to_string().contains("job_not_found") && resubmits < MAX_JOB_NOT_FOUND_RESUBMITS => {
                // Job not found - resubmit
                resubmits += 1;
                warn!(
                    "{} proof job {} not found (batch {}), resubmitting attempt {}/{}",
                    proof_type, current_job, batch_idx, resubmits, MAX_JOB_NOT_FOUND_RESUBMITS
                );

                match client.submit_proof_async(inputs_json.clone(), proof_type).await {
                    Ok(new_job_id) => {
                        info!(
                            "Batch {} ({}) resubmitted with job_id {}",
                            batch_idx, proof_type, new_job_id
                        );
                        current_job = new_job_id;
                        continue;
                    }
                    Err(submit_err) => {
                        return Err(anyhow::anyhow!(
                            "Failed to resubmit {} proof for batch {}: {}",
                            proof_type,
                            batch_idx,
                            submit_err
                        ));
                    }
                }
            }
            Err(e) => {
                // Other error - return immediately
                return Err(anyhow::anyhow!(
                    "{} proof polling failed for batch {}: {}",
                    proof_type,
                    batch_idx,
                    e
                ));
            }
        }
    }
}
