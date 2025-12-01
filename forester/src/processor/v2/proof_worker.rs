use std::sync::Arc;
use std::time::Duration;

use async_channel::Receiver;
use light_batched_merkle_tree::merkle_tree::{
    InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
};
use light_prover_client::{
    proof::ProofCompressed,
    proof_client::{ProofClient, SubmitProofResult},
    proof_types::{
        batch_address_append::{to_json as address_append_to_json, BatchAddressAppendInputs},
        batch_append::{BatchAppendInputsJson, BatchAppendsCircuitInputs},
        batch_update::{update_inputs_string, BatchUpdateCircuitInputs},
    },
};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

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

    fn to_json(&self) -> String {
        match self {
            ProofInput::Append(inputs) => BatchAppendInputsJson::from_inputs(inputs).to_string(),
            ProofInput::Nullify(inputs) => update_inputs_string(inputs),
            ProofInput::AddressAppend(inputs) => address_append_to_json(inputs),
        }
    }

    fn new_root_bytes(&self) -> crate::Result<[u8; 32]> {
        match self {
            ProofInput::Append(inputs) => light_hasher::bigint::bigint_to_be_bytes_array::<32>(
                &inputs.new_root.to_biguint().unwrap(),
            )
            .map_err(Into::into),
            ProofInput::Nullify(inputs) => light_hasher::bigint::bigint_to_be_bytes_array::<32>(
                &inputs.new_root.to_biguint().unwrap(),
            )
            .map_err(Into::into),
            ProofInput::AddressAppend(inputs) => {
                light_hasher::bigint::bigint_to_be_bytes_array::<32>(&inputs.new_root)
                    .map_err(Into::into)
            }
        }
    }
}

pub struct ProofJob {
    pub(crate) seq: u64,
    pub(crate) inputs: ProofInput,
    pub(crate) result_tx: mpsc::Sender<ProofResult>,
}

#[derive(Debug)]
pub struct ProofResult {
    pub(crate) seq: u64,
    pub(crate) result: Result<BatchInstruction, String>,
}

/// Shared proof clients for all workers
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
            ),
            nullify_client: ProofClient::with_config(
                config.update_url.clone(),
                config.polling_interval,
                config.max_wait_time,
                config.api_key.clone(),
            ),
            address_append_client: ProofClient::with_config(
                config.address_append_url.clone(),
                config.polling_interval,
                config.max_wait_time,
                config.api_key.clone(),
            ),
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

/// A pending proof job that has been submitted but not yet completed
struct PendingProof {
    seq: u64,
    job_id: String,
    inputs: ProofInput,
    result_tx: mpsc::Sender<ProofResult>,
}

pub fn spawn_proof_workers(
    num_workers: usize,
    config: &ProverConfig,
) -> async_channel::Sender<ProofJob> {
    let num_workers = if num_workers == 0 {
        warn!("spawn_proof_workers called with num_workers=0, using 1 instead");
        1
    } else {
        num_workers
    };

    let channel_capacity = num_workers * 4;
    let (job_tx, job_rx) = async_channel::bounded::<ProofJob>(channel_capacity);

    let clients = Arc::new(ProofClients::new(config));
    let polling_interval = config.polling_interval;

    tokio::spawn(async move {
        run_async_proof_orchestrator(job_rx, clients, polling_interval, num_workers).await
    });

    info!(
        "Spawned async proof orchestrator with {} concurrent pollers",
        num_workers
    );
    job_tx
}

async fn run_async_proof_orchestrator(
    job_rx: Receiver<ProofJob>,
    clients: Arc<ProofClients>,
    polling_interval: Duration,
    max_concurrent_polls: usize,
) -> crate::Result<()> {
    let (poll_tx, mut poll_rx) = mpsc::channel::<PendingProof>(max_concurrent_polls * 2);

    let clients_for_submit = clients.clone();
    let submit_handle =
        tokio::spawn(async move { run_submission_loop(job_rx, clients_for_submit, poll_tx).await });

    let mut poll_handles = Vec::with_capacity(max_concurrent_polls);
    let (pending_tx, pending_rx) = async_channel::bounded::<PendingProof>(max_concurrent_polls * 4);

    for worker_id in 0..max_concurrent_polls {
        let pending_rx = pending_rx.clone();
        let clients = clients.clone();
        let polling_interval = polling_interval;

        let handle = tokio::spawn(async move {
            run_poll_worker(worker_id, pending_rx, clients, polling_interval).await
        });
        poll_handles.push(handle);
    }

    while let Some(pending) = poll_rx.recv().await {
        if pending_tx.send(pending).await.is_err() {
            warn!("Poll worker channel closed, stopping orchestrator");
            break;
        }
    }

    drop(pending_tx);

    let _ = submit_handle.await;

    for handle in poll_handles {
        let _ = handle.await;
    }

    Ok(())
}

async fn run_submission_loop(
    job_rx: Receiver<ProofJob>,
    clients: Arc<ProofClients>,
    poll_tx: mpsc::Sender<PendingProof>,
) -> crate::Result<()> {
    while let Ok(job) = job_rx.recv().await {
        let client = clients.get_client(&job.inputs);
        let inputs_json = job.inputs.to_json();
        let circuit_type = job.inputs.circuit_type();

        match client.submit_proof_async(inputs_json, circuit_type).await {
            Ok(SubmitProofResult::Queued(job_id)) => {
                debug!(
                    "Submitted proof job seq={} type={} job_id={}",
                    job.seq, circuit_type, job_id
                );

                let pending = PendingProof {
                    seq: job.seq,
                    job_id,
                    inputs: job.inputs,
                    result_tx: job.result_tx,
                };

                if poll_tx.send(pending).await.is_err() {
                    warn!("Poll channel closed, stopping submission loop");
                    break;
                }
            }
            Ok(SubmitProofResult::Immediate(proof)) => {
                debug!(
                    "Got immediate proof for seq={} type={}",
                    job.seq, circuit_type
                );

                let result = build_proof_result(job.seq, &job.inputs, proof);

                if job.result_tx.send(result).await.is_err() {
                    debug!("Result channel closed for job seq={}", job.seq);
                }
            }
            Err(e) => {
                error!(
                    "Failed to submit proof job seq={} type={}: {}",
                    job.seq, circuit_type, e
                );

                let result = ProofResult {
                    seq: job.seq,
                    result: Err(format!("Submit failed: {}", e)),
                };

                if job.result_tx.send(result).await.is_err() {
                    debug!("Result channel closed for job seq={}", job.seq);
                }
            }
        }
    }

    Ok(())
}

async fn run_poll_worker(
    worker_id: usize,
    pending_rx: async_channel::Receiver<PendingProof>,
    clients: Arc<ProofClients>,
    polling_interval: Duration,
) -> crate::Result<()> {
    while let Ok(pending) = pending_rx.recv().await {
        let client = clients.get_client(&pending.inputs);

        debug!(
            "Poll worker {} polling job_id={} seq={}",
            worker_id, pending.job_id, pending.seq
        );

        let result = poll_and_build_result(client, &pending, polling_interval).await;

        if pending.result_tx.send(result).await.is_err() {
            debug!(
                "Result channel closed for job seq={}, continuing",
                pending.seq
            );
        }
    }

    debug!("Poll worker {} shutting down", worker_id);
    Ok(())
}

fn build_proof_result(seq: u64, inputs: &ProofInput, proof: ProofCompressed) -> ProofResult {
    let new_root = match inputs.new_root_bytes() {
        Ok(root) => root,
        Err(e) => {
            return ProofResult {
                seq,
                result: Err(format!("Failed to get new root: {}", e)),
            };
        }
    };

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

    ProofResult {
        seq,
        result: Ok(instruction),
    }
}

async fn poll_and_build_result(
    client: &ProofClient,
    pending: &PendingProof,
    _polling_interval: Duration,
) -> ProofResult {
    match client.poll_proof_completion(pending.job_id.clone()).await {
        Ok(proof) => {
            debug!("Proof completed for seq={}", pending.seq);
            build_proof_result(pending.seq, &pending.inputs, proof)
        }
        Err(e) => {
            warn!(
                "Proof polling failed for seq={} job_id={}: {}",
                pending.seq, pending.job_id, e
            );

            ProofResult {
                seq: pending.seq,
                result: Err(format!("Proof failed: {}", e)),
            }
        }
    }
}
