use std::sync::Arc;

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
        let client = clients.get_client(&job.inputs);
        let inputs_json = job.inputs.to_json();
        let circuit_type = job.inputs.circuit_type();

        match client.submit_proof_async(inputs_json, circuit_type).await {
            Ok(SubmitProofResult::Queued(job_id)) => {
                debug!(
                    "Submitted proof job seq={} type={} job_id={}",
                    job.seq, circuit_type, job_id
                );

                let poll_client = clients.clone();
                tokio::spawn(async move {
                    poll_and_send_result(poll_client, job_id, job.seq, job.inputs, job.result_tx)
                        .await
                });
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
                };
                let _ = job.result_tx.send(result).await;
            }
        }
    }

    Ok(())
}

async fn poll_and_send_result(
    clients: Arc<ProofClients>,
    job_id: String,
    seq: u64,
    inputs: ProofInput,
    result_tx: mpsc::Sender<ProofResult>,
) {
    let client = clients.get_client(&inputs);

    let result = match client.poll_proof_completion(job_id.clone()).await {
        Ok(proof) => {
            debug!("Proof completed for seq={} job_id={}", seq, job_id);
            build_proof_result(seq, &inputs, proof)
        }
        Err(e) => {
            warn!(
                "Proof polling failed for seq={} job_id={}: {}",
                seq, job_id, e
            );
            ProofResult {
                seq,
                result: Err(format!("Proof failed: {}", e)),
            }
        }
    };

    if result_tx.send(result).await.is_err() {
        debug!("Result channel closed for job seq={}", seq);
    }
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
