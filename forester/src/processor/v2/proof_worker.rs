use std::sync::Arc;

use async_channel::Receiver;
use light_batched_merkle_tree::merkle_tree::{
    InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
};
use light_prover_client::{
    errors::ProverClientError,
    proof::ProofCompressed,
    proof_client::{ProofClient, SubmitProofResult},
    proof_types::{
        batch_address_append::{to_json as address_append_to_json, BatchAddressAppendInputs},
        batch_append::{BatchAppendInputsJson, BatchAppendsCircuitInputs},
        batch_update::{update_inputs_string, BatchUpdateCircuitInputs},
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

    fn to_json(&self) -> String {
        match self {
            ProofInput::Append(inputs) => BatchAppendInputsJson::from_inputs(inputs).to_string(),
            ProofInput::Nullify(inputs) => update_inputs_string(inputs),
            ProofInput::AddressAppend(inputs) => address_append_to_json(inputs),
        }
    }

    fn new_root_bytes(&self) -> crate::Result<[u8; 32]> {
        match self {
            ProofInput::Append(inputs) => {
                let biguint = inputs
                    .new_root
                    .to_biguint()
                    .ok_or_else(|| anyhow::anyhow!("Failed to convert append new_root to biguint"))?;
                light_hasher::bigint::bigint_to_be_bytes_array::<32>(&biguint).map_err(Into::into)
            }
            ProofInput::Nullify(inputs) => {
                let biguint = inputs
                    .new_root
                    .to_biguint()
                    .ok_or_else(|| anyhow::anyhow!("Failed to convert nullify new_root to biguint"))?;
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
                let biguint = inputs
                    .old_root
                    .to_biguint()
                    .ok_or_else(|| anyhow::anyhow!("Failed to convert append old_root to biguint"))?;
                light_hasher::bigint::bigint_to_be_bytes_array::<32>(&biguint).map_err(Into::into)
            }
            ProofInput::Nullify(inputs) => {
                let biguint = inputs
                    .old_root
                    .to_biguint()
                    .ok_or_else(|| anyhow::anyhow!("Failed to convert nullify old_root to biguint"))?;
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
    pub(crate) result_tx: mpsc::Sender<ProofResult>,
}

#[derive(Debug)]
pub struct ProofResult {
    pub(crate) seq: u64,
    pub(crate) result: Result<BatchInstruction, String>,
    pub(crate) old_root: [u8; 32],
    pub(crate) new_root: [u8; 32],
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

pub fn spawn_proof_workers(config: &ProverConfig) -> async_channel::Sender<ProofJob> {
    let (job_tx, job_rx) = async_channel::bounded::<ProofJob>(256);
    let clients = Arc::new(ProofClients::new(config));
    tokio::spawn(async move { run_proof_pipeline(job_rx, clients).await });
    job_tx
}

async fn run_proof_pipeline(
    job_rx: Receiver<ProofJob>,
    clients: Arc<ProofClients>,
) -> crate::Result<()> {
    while let Ok(job) = job_rx.recv().await {
        let clients = clients.clone();
        // Spawn immediately so we don't block receiving the next job
        // while waiting for HTTP submission
        tokio::spawn(async move {
            submit_and_poll_proof(clients, job).await;
        });
    }

    Ok(())
}

async fn submit_and_poll_proof(clients: Arc<ProofClients>, job: ProofJob) {
    let client = clients.get_client(&job.inputs);
    let inputs_json = job.inputs.to_json();
    let circuit_type = job.inputs.circuit_type();

    match client.submit_proof_async(inputs_json, circuit_type).await {
        Ok(SubmitProofResult::Queued(job_id)) => {
            debug!(
                "Submitted proof job seq={} type={} job_id={}",
                job.seq, circuit_type, job_id
            );

            poll_and_send_result(clients, job_id, job.seq, job.inputs, job.result_tx).await;
        }
        Ok(SubmitProofResult::Immediate(proof)) => {
            debug!(
                "Got immediate proof for seq={} type={}",
                job.seq, circuit_type
            );

            let result = build_proof_result(job.seq, &job.inputs, proof);
            let _ = job.result_tx.send(result).await;
        }
        Err(e) => {
            error!(
                "Failed to submit proof job seq={} type={}: {}",
                job.seq, circuit_type, e
            );

            let result = ProofResult {
                seq: job.seq,
                result: Err(format!("Submit failed: {}", e)),
                old_root: [0u8; 32],
                new_root: [0u8; 32],
            };
            let _ = job.result_tx.send(result).await;
        }
    }
}

async fn poll_and_send_result(
    clients: Arc<ProofClients>,
    job_id: String,
    seq: u64,
    inputs: ProofInput,
    result_tx: mpsc::Sender<ProofResult>,
) {
    let client = clients.get_client(&inputs);

    // Poll; on job_not_found, resubmit once and poll the new job.
    let result = match client.poll_proof_completion(job_id.clone()).await {
        Ok(proof) => {
            debug!("Proof completed for seq={} job_id={}", seq, job_id);
            build_proof_result(seq, &inputs, proof)
        }
        Err(e) if is_job_not_found(&e) => {
            warn!(
                "Proof polling got job_not_found for seq={} job_id={}; retrying submit once",
                seq, job_id
            );
            let inputs_json = inputs.to_json();
            let circuit_type = inputs.circuit_type();
            match client.submit_proof_async(inputs_json, circuit_type).await {
                Ok(SubmitProofResult::Queued(new_job_id)) => {
                    debug!(
                        "Resubmitted proof job seq={} type={} new_job_id={}",
                        seq, circuit_type, new_job_id
                    );
                    match client.poll_proof_completion(new_job_id.clone()).await {
                        Ok(proof) => {
                            debug!(
                                "Proof completed after retry for seq={} job_id={}",
                                seq, new_job_id
                            );
                            build_proof_result(seq, &inputs, proof)
                        }
                        Err(e2) => ProofResult {
                            seq,
                            result: Err(format!(
                                "Proof failed after retry job_id={}: {}",
                                new_job_id, e2
                            )),
                            old_root: [0u8; 32],
                            new_root: [0u8; 32],
                        },
                    }
                }
                Ok(SubmitProofResult::Immediate(proof)) => {
                    debug!(
                        "Immediate proof after retry for seq={} type={}",
                        seq, circuit_type
                    );
                    build_proof_result(seq, &inputs, proof)
                }
                Err(e_submit) => ProofResult {
                    seq,
                    result: Err(format!("Proof retry submit failed: {}", e_submit)),
                    old_root: [0u8; 32],
                    new_root: [0u8; 32],
                },
            }
        }
        Err(e) => {
            warn!(
                "Proof polling failed for seq={} job_id={}: {}",
                seq, job_id, e
            );
            ProofResult {
                seq,
                result: Err(format!("Proof failed: {}", e)),
                old_root: [0u8; 32],
                new_root: [0u8; 32],
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

fn build_proof_result(seq: u64, inputs: &ProofInput, proof: ProofCompressed) -> ProofResult {
    let new_root = match inputs.new_root_bytes() {
        Ok(root) => root,
        Err(e) => {
            return ProofResult {
                seq,
                result: Err(format!("Failed to get new root: {}", e)),
                old_root: [0u8; 32],
                new_root: [0u8; 32],
            };
        }
    };
    let old_root = match inputs.old_root_bytes() {
        Ok(root) => root,
        Err(e) => {
            return ProofResult {
                seq,
                result: Err(format!("Failed to get old root: {}", e)),
                old_root: [0u8; 32],
                new_root: [0u8; 32],
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
        old_root,
        new_root,
        result: Ok(instruction),
    }
}
