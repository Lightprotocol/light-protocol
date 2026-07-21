use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

#[cfg(not(test))]
const QUEUE_FULL_RETRY_DELAY: Duration = Duration::from_secs(5);
#[cfg(test)]
const QUEUE_FULL_RETRY_DELAY: Duration = Duration::from_millis(10);

use async_channel::Receiver;
use light_batched_merkle_tree::merkle_tree::{
    InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
};
use light_prover_client::{
    errors::ProverClientError,
    proof::ProofResult,
    proof_client::{ProofClient, SubmitProofResult},
    proof_types::{
        batch_address_append::BatchAddressAppendInputs,
        batch_append::{BatchAppendInputsJson, BatchAppendsCircuitInputs},
        batch_update::BatchUpdateCircuitInputs,
    },
};
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

use crate::processor::v2::{tx_sender::BatchInstruction, ProverConfig};

#[derive(Debug, Clone)]
pub enum ProofInput {
    Append(BatchAppendsCircuitInputs),
    Nullify(BatchUpdateCircuitInputs),
    AddressAppend(BatchAddressAppendInputs),
}

impl ProofInput {
    fn circuit_type(&self) -> &'static str {
        match self {
            ProofInput::Append(_) => "append",
            ProofInput::Nullify(_) => "update",
            ProofInput::AddressAppend(_) => "address_append",
        }
    }

    fn to_json(&self, tree_id: &str, batch_index: u64) -> String {
        match self {
            ProofInput::Append(inputs) => BatchAppendInputsJson::from_inputs(inputs)
                .with_tree_id(tree_id.to_string())
                .with_batch_index(batch_index)
                .to_string(),
            ProofInput::Nullify(inputs) => {
                use light_prover_client::proof_types::batch_update::BatchUpdateProofInputsJson;
                BatchUpdateProofInputsJson::from_update_inputs(inputs)
                    .with_tree_id(tree_id.to_string())
                    .with_batch_index(batch_index)
                    .to_string()
            }
            ProofInput::AddressAppend(inputs) => {
                use light_prover_client::proof_types::batch_address_append::BatchAddressAppendInputsJson;
                BatchAddressAppendInputsJson::from_inputs(inputs)
                    .with_tree_id(tree_id.to_string())
                    .with_batch_index(batch_index)
                    .to_string()
            }
        }
    }

    fn new_root_bytes(&self) -> crate::Result<[u8; 32]> {
        match self {
            ProofInput::Append(inputs) => {
                let biguint = inputs.new_root.to_biguint().ok_or_else(|| {
                    anyhow::anyhow!("Failed to convert append new_root to biguint")
                })?;
                light_hasher::bigint::bigint_to_be_bytes_array::<32>(&biguint).map_err(Into::into)
            }
            ProofInput::Nullify(inputs) => {
                let biguint = inputs.new_root.to_biguint().ok_or_else(|| {
                    anyhow::anyhow!("Failed to convert nullify new_root to biguint")
                })?;
                light_hasher::bigint::bigint_to_be_bytes_array::<32>(&biguint).map_err(Into::into)
            }
            ProofInput::AddressAppend(inputs) => {
                light_hasher::bigint::bigint_to_be_bytes_array::<32>(&inputs.new_root)
                    .map_err(Into::into)
            }
        }
    }

    fn old_root_bytes(&self) -> crate::Result<[u8; 32]> {
        match self {
            ProofInput::Append(inputs) => {
                let biguint = inputs.old_root.to_biguint().ok_or_else(|| {
                    anyhow::anyhow!("Failed to convert append old_root to biguint")
                })?;
                light_hasher::bigint::bigint_to_be_bytes_array::<32>(&biguint).map_err(Into::into)
            }
            ProofInput::Nullify(inputs) => {
                let biguint = inputs.old_root.to_biguint().ok_or_else(|| {
                    anyhow::anyhow!("Failed to convert nullify old_root to biguint")
                })?;
                light_hasher::bigint::bigint_to_be_bytes_array::<32>(&biguint).map_err(Into::into)
            }
            ProofInput::AddressAppend(inputs) => {
                light_hasher::bigint::bigint_to_be_bytes_array::<32>(&inputs.old_root)
                    .map_err(Into::into)
            }
        }
    }
}

pub struct ProofJob {
    pub(crate) seq: u64,
    pub(crate) inputs: ProofInput,
    pub(crate) result_tx: mpsc::Sender<ProofJobResult>,
    /// Tree pubkey for fair queuing - used to prevent starvation when multiple trees have proofs pending
    pub(crate) tree_id: String,
}

#[derive(Debug)]
pub struct ProofJobResult {
    pub(crate) seq: u64,
    pub(crate) result: Result<BatchInstruction, String>,
    pub(crate) old_root: [u8; 32],
    pub(crate) new_root: [u8; 32],
    /// Pure proof generation time in milliseconds (from prover server).
    pub(crate) proof_duration_ms: u64,
    /// Total round-trip time in milliseconds (submit to result, includes queue wait).
    pub(crate) round_trip_ms: u64,
    /// When this proof job was submitted (for end-to-end latency tracking).
    pub(crate) submitted_at: std::time::Instant,
}

struct ProofClients {
    append_client: ProofClient,
    nullify_client: ProofClient,
    address_append_client: ProofClient,
}

impl ProofClients {
    fn new(config: &ProverConfig) -> Self {
        Self {
            append_client: ProofClient::with_config(
                config.append_url.clone(),
                config.polling_interval,
                config.max_wait_time,
                config.api_key.clone(),
            )
            .with_network(config.network.clone()),
            nullify_client: ProofClient::with_config(
                config.update_url.clone(),
                config.polling_interval,
                config.max_wait_time,
                config.api_key.clone(),
            )
            .with_network(config.network.clone()),
            address_append_client: ProofClient::with_config(
                config.address_append_url.clone(),
                config.polling_interval,
                config.max_wait_time,
                config.api_key.clone(),
            )
            .with_network(config.network.clone()),
        }
    }

    fn get_client(&self, input: &ProofInput) -> &ProofClient {
        match input {
            ProofInput::Append(_) => &self.append_client,
            ProofInput::Nullify(_) => &self.nullify_client,
            ProofInput::AddressAppend(_) => &self.address_append_client,
        }
    }
}

pub fn spawn_proof_workers(config: &ProverConfig) -> async_channel::Sender<ProofJob> {
    let (job_tx, job_rx) = async_channel::bounded::<ProofJob>(256);
    let clients = Arc::new(ProofClients::new(config));
    let semaphore = process_proof_semaphore(config.max_concurrent_jobs);
    tokio::spawn(async move { run_proof_pipeline(job_rx, clients, semaphore).await });
    job_tx
}

fn process_proof_semaphore(max_concurrent_jobs: usize) -> Arc<tokio::sync::Semaphore> {
    static SEMAPHORE: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
    SEMAPHORE
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(max_concurrent_jobs.max(1))))
        .clone()
}

async fn run_proof_pipeline(
    job_rx: Receiver<ProofJob>,
    clients: Arc<ProofClients>,
    semaphore: Arc<tokio::sync::Semaphore>,
) -> crate::Result<()> {
    while let Ok(job) = job_rx.recv().await {
        let clients = clients.clone();
        let permit = semaphore.clone().acquire_owned().await;
        // Spawn immediately so we don't block receiving the next job
        // while waiting for HTTP submission. Semaphore bounds concurrency.
        tokio::spawn(async move {
            let _permit = permit;
            submit_and_poll_proof(clients, job).await;
        });
    }

    Ok(())
}

async fn submit_and_poll_proof(clients: Arc<ProofClients>, job: ProofJob) {
    let client = clients.get_client(&job.inputs);
    // Use seq as batch_index for ordering in the prover queue
    let inputs_json = job.inputs.to_json(&job.tree_id, job.seq);
    let circuit_type = job.inputs.circuit_type();

    let round_trip_start = std::time::Instant::now();

    let Some(submit_result) =
        submit_with_backpressure(client, &inputs_json, circuit_type, job.seq, &job.result_tx).await
    else {
        return;
    };

    match submit_result {
        Ok(SubmitProofResult::Queued(job_id)) => {
            debug!(
                "Submitted proof job seq={} type={} job_id={}",
                job.seq, circuit_type, job_id
            );

            poll_and_send_result(
                clients,
                job_id,
                job.seq,
                job.inputs,
                job.tree_id,
                job.result_tx,
                round_trip_start,
            )
            .await;
        }
        Ok(SubmitProofResult::Immediate(proof)) => {
            let round_trip_ms = round_trip_start.elapsed().as_millis() as u64;
            debug!(
                "Got immediate proof for seq={} type={} round_trip={}ms",
                job.seq, circuit_type, round_trip_ms
            );

            let result =
                build_proof_result(job.seq, &job.inputs, proof, round_trip_ms, round_trip_start);
            let _ = job.result_tx.send(result).await;
        }
        Err(e) => {
            error!(
                "Failed to submit proof job seq={} type={}: {}",
                job.seq, circuit_type, e
            );

            let result = ProofJobResult {
                seq: job.seq,
                result: Err(format!("Submit failed: {}", e)),
                old_root: [0u8; 32],
                new_root: [0u8; 32],
                proof_duration_ms: 0,
                round_trip_ms: 0,
                submitted_at: round_trip_start,
            };
            let _ = job.result_tx.send(result).await;
        }
    }
}

async fn submit_with_backpressure(
    client: &ProofClient,
    inputs_json: &str,
    circuit_type: &str,
    seq: u64,
    result_tx: &mpsc::Sender<ProofJobResult>,
) -> Option<Result<SubmitProofResult, ProverClientError>> {
    let mut attempts = 0u64;
    loop {
        if result_tx.is_closed() {
            debug!(
                "Stopping proof submission retry for seq={}: result channel closed",
                seq
            );
            return None;
        }

        match client
            .submit_proof_async(inputs_json.to_string(), circuit_type)
            .await
        {
            Err(error) if is_queue_full(&error) => {
                attempts += 1;
                let jitter = Duration::from_millis(seq.wrapping_add(attempts) % 1000);
                let delay = QUEUE_FULL_RETRY_DELAY + jitter;
                warn!(
                    "Prover queue full for seq={} type={}; retrying in {:?} (attempt {})",
                    seq, circuit_type, delay, attempts
                );
                tokio::select! {
                    _ = tokio::time::sleep(delay) => {}
                    _ = result_tx.closed() => {
                        debug!(
                            "Stopping proof submission retry for seq={}: result channel closed",
                            seq
                        );
                        return None;
                    }
                }
            }
            result => return Some(result),
        }
    }
}

fn is_queue_full(err: &ProverClientError) -> bool {
    matches!(
        err,
        ProverClientError::ProverServerError(message) if message.contains("queue_full")
    )
}

async fn poll_and_send_result(
    clients: Arc<ProofClients>,
    job_id: String,
    seq: u64,
    inputs: ProofInput,
    tree_id: String,
    result_tx: mpsc::Sender<ProofJobResult>,
    round_trip_start: std::time::Instant,
) {
    let client = clients.get_client(&inputs);

    // Poll; on job_not_found, resubmit once and poll the new job.
    let result = match client.poll_proof_completion(job_id.clone()).await {
        Ok(proof) => {
            let round_trip_ms = round_trip_start.elapsed().as_millis() as u64;
            debug!(
                "Proof completed for seq={} job_id={} round_trip={}ms proof={}ms",
                seq, job_id, round_trip_ms, proof.proof_duration_ms
            );
            build_proof_result(seq, &inputs, proof, round_trip_ms, round_trip_start)
        }
        Err(e) if is_job_not_found(&e) => {
            warn!(
                "Proof polling got job_not_found for seq={} job_id={}; retrying submit once",
                seq, job_id
            );
            tokio::time::sleep(Duration::from_millis(200)).await;

            let inputs_json = inputs.to_json(&tree_id, seq);
            let circuit_type = inputs.circuit_type();
            let Some(submit_result) =
                submit_with_backpressure(client, &inputs_json, circuit_type, seq, &result_tx).await
            else {
                return;
            };

            match submit_result {
                Ok(SubmitProofResult::Queued(new_job_id)) => {
                    debug!(
                        "Resubmitted proof job seq={} type={} new_job_id={}",
                        seq, circuit_type, new_job_id
                    );
                    match client.poll_proof_completion(new_job_id.clone()).await {
                        Ok(proof) => {
                            let round_trip_ms = round_trip_start.elapsed().as_millis() as u64;
                            debug!(
                                "Proof completed after retry for seq={} job_id={} round_trip={}ms",
                                seq, new_job_id, round_trip_ms
                            );
                            build_proof_result(seq, &inputs, proof, round_trip_ms, round_trip_start)
                        }
                        Err(e2) => ProofJobResult {
                            seq,
                            result: Err(format!(
                                "Proof failed after retry job_id={}: {}",
                                new_job_id, e2
                            )),
                            old_root: [0u8; 32],
                            new_root: [0u8; 32],
                            proof_duration_ms: 0,
                            round_trip_ms: 0,
                            submitted_at: round_trip_start,
                        },
                    }
                }
                Ok(SubmitProofResult::Immediate(proof)) => {
                    let round_trip_ms = round_trip_start.elapsed().as_millis() as u64;
                    debug!(
                        "Immediate proof after retry for seq={} type={} round_trip={}ms",
                        seq, circuit_type, round_trip_ms
                    );
                    build_proof_result(seq, &inputs, proof, round_trip_ms, round_trip_start)
                }
                Err(e_submit) => ProofJobResult {
                    seq,
                    result: Err(format!("Proof retry submit failed: {}", e_submit)),
                    old_root: [0u8; 32],
                    new_root: [0u8; 32],
                    proof_duration_ms: 0,
                    round_trip_ms: 0,
                    submitted_at: round_trip_start,
                },
            }
        }
        Err(e) => {
            warn!(
                "Proof polling failed for seq={} job_id={}: {}",
                seq, job_id, e
            );
            ProofJobResult {
                seq,
                result: Err(format!("Proof failed: {}", e)),
                old_root: [0u8; 32],
                new_root: [0u8; 32],
                proof_duration_ms: 0,
                round_trip_ms: 0,
                submitted_at: round_trip_start,
            }
        }
    };

    if result_tx.send(result).await.is_err() {
        debug!("Result channel closed for job seq={}", seq);
    }
}

fn is_job_not_found(err: &ProverClientError) -> bool {
    matches!(
        err,
        ProverClientError::ProverServerError(msg) if msg.contains("job_not_found")
    )
}

fn build_proof_result(
    seq: u64,
    inputs: &ProofInput,
    proof_with_timing: ProofResult,
    round_trip_ms: u64,
    submitted_at: std::time::Instant,
) -> ProofJobResult {
    let new_root = match inputs.new_root_bytes() {
        Ok(root) => root,
        Err(e) => {
            return ProofJobResult {
                seq,
                result: Err(format!("Failed to get new root: {}", e)),
                old_root: [0u8; 32],
                new_root: [0u8; 32],
                proof_duration_ms: proof_with_timing.proof_duration_ms,
                round_trip_ms,
                submitted_at,
            };
        }
    };
    let old_root = match inputs.old_root_bytes() {
        Ok(root) => root,
        Err(e) => {
            return ProofJobResult {
                seq,
                result: Err(format!("Failed to get old root: {}", e)),
                old_root: [0u8; 32],
                new_root: [0u8; 32],
                proof_duration_ms: proof_with_timing.proof_duration_ms,
                round_trip_ms,
                submitted_at,
            };
        }
    };

    let proof = proof_with_timing.proof;
    let instruction = match inputs {
        ProofInput::Append(_) => BatchInstruction::Append(vec![InstructionDataBatchAppendInputs {
            new_root,
            compressed_proof: proof.into(),
        }]),
        ProofInput::Nullify(_) => {
            BatchInstruction::Nullify(vec![InstructionDataBatchNullifyInputs {
                new_root,
                compressed_proof: proof.into(),
            }])
        }
        ProofInput::AddressAppend(_) => BatchInstruction::AddressAppend(vec![
            light_batched_merkle_tree::merkle_tree::InstructionDataAddressAppendInputs {
                new_root,
                compressed_proof: proof.into(),
            },
        ]),
    };

    ProofJobResult {
        seq,
        old_root,
        new_root,
        result: Ok(instruction),
        proof_duration_ms: proof_with_timing.proof_duration_ms,
        round_trip_ms,
        submitted_at,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::*;

    #[test]
    fn queue_full_detection_is_specific() {
        let queue_full = ProverClientError::ProverServerError(
            "Prover server error: queue_full - network proof queue is full".to_string(),
        );
        let other = ProverClientError::ProverServerError("invalid_input".to_string());

        assert!(is_queue_full(&queue_full));
        assert!(!is_queue_full(&other));
    }

    #[tokio::test]
    async fn retries_queue_full_until_job_is_accepted() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let request_count = Arc::new(AtomicUsize::new(0));
        let server_request_count = request_count.clone();

        let server = tokio::spawn(async move {
            for request_index in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = vec![0u8; 16 * 1024];
                let _ = stream.read(&mut request).await.unwrap();
                server_request_count.fetch_add(1, Ordering::SeqCst);

                let (status, body) = if request_index == 0 {
                    (
                        "429 Too Many Requests",
                        r#"{"code":"queue_full","message":"network proof queue is full"}"#,
                    )
                } else {
                    (
                        "202 Accepted",
                        r#"{"job_id":"job-1","estimated_time":null}"#,
                    )
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        });

        let client = ProofClient::with_config(
            format!("http://{address}"),
            Duration::from_millis(10),
            Duration::from_secs(1),
            None,
        );
        let (result_tx, _result_rx) = mpsc::channel(1);

        let submit_result = tokio::time::timeout(
            Duration::from_secs(2),
            submit_with_backpressure(&client, "{}", "append", 7, &result_tx),
        )
        .await
        .expect("submission retry timed out")
        .expect("result channel unexpectedly closed")
        .expect("submission failed after queue became available");

        match submit_result {
            SubmitProofResult::Queued(job_id) => assert_eq!(job_id, "job-1"),
            SubmitProofResult::Immediate(_) => panic!("expected queued proof job"),
        }
        server.await.unwrap();
        assert_eq!(request_count.load(Ordering::SeqCst), 2);
    }
}
