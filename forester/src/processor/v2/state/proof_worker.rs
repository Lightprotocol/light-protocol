use async_channel::Receiver;
use light_batched_merkle_tree::merkle_tree::{
    InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
};
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::{
        batch_address_append::BatchAddressAppendInputs, batch_append::BatchAppendsCircuitInputs,
        batch_update::BatchUpdateCircuitInputs,
    },
};
use tokio::sync::mpsc;
use tracing::{debug, info, trace, warn};

use crate::processor::v2::{state::tx_sender::BatchInstruction, ProverConfig};

#[derive(Debug)]
pub enum ProofInput {
    Append(BatchAppendsCircuitInputs),
    Nullify(BatchUpdateCircuitInputs),
    AddressAppend(BatchAddressAppendInputs),
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

pub fn spawn_proof_workers(
    num_workers: usize,
    config: ProverConfig,
) -> async_channel::Sender<ProofJob> {
    // Enforce minimum of 1 worker to prevent zero-capacity channels and no workers
    let num_workers = if num_workers == 0 {
        warn!("spawn_proof_workers called with num_workers=0, using 1 instead");
        1
    } else {
        num_workers
    };

    let channel_capacity = num_workers * 2;
    let (job_tx, job_rx) = async_channel::bounded::<ProofJob>(channel_capacity);

    for worker_id in 0..num_workers {
        let job_rx = job_rx.clone();
        let config = config.clone();
        tokio::spawn(async move { run_proof_worker(worker_id, job_rx, config).await });
    }

    info!("Spawned {} proof workers", num_workers);
    job_tx
}

async fn run_proof_worker(
    worker_id: usize,
    job_rx: Receiver<ProofJob>,
    config: ProverConfig,
) -> crate::Result<()> {
    let append_client = ProofClient::with_config(
        config.append_url,
        config.polling_interval,
        config.max_wait_time,
        config.api_key.clone(),
    );
    let nullify_client = ProofClient::with_config(
        config.update_url,
        config.polling_interval,
        config.max_wait_time,
        config.api_key,
    );

    trace!("ProofWorker {} started", worker_id);

    while let Ok(job) = job_rx.recv().await {
        debug!("ProofWorker {} processing job seq={}", worker_id, job.seq);

        let result_data = match job.inputs {
            ProofInput::Append(inputs) => {
                match append_client.generate_batch_append_proof(inputs).await {
                    Ok((proof, new_root)) => Ok(BatchInstruction::Append(vec![
                        InstructionDataBatchAppendInputs {
                            new_root,
                            compressed_proof: proof.into(),
                        },
                    ])),
                    Err(e) => {
                        warn!("ProofWorker {} append proof failed: {}", worker_id, e);
                        Err(format!("Append proof failed: {}", e))
                    }
                }
            }
            ProofInput::Nullify(inputs) => {
                match nullify_client.generate_batch_update_proof(inputs).await {
                    Ok((proof, new_root)) => Ok(BatchInstruction::Nullify(vec![
                        InstructionDataBatchNullifyInputs {
                            new_root,
                            compressed_proof: proof.into(),
                        },
                    ])),
                    Err(e) => {
                        warn!("ProofWorker {} nullify proof failed: {}", worker_id, e);
                        Err(format!("Nullify proof failed: {}", e))
                    }
                }
            }
            ProofInput::AddressAppend(inputs) => {
                match append_client.generate_batch_address_append_proof(inputs).await {
                    Ok((proof, new_root)) => Ok(BatchInstruction::AddressAppend(vec![
                        light_batched_merkle_tree::merkle_tree::InstructionDataAddressAppendInputs {
                            new_root,
                            compressed_proof: proof.into(),
                        },
                    ])),
                    Err(e) => {
                        warn!(
                            "ProofWorker {} address append proof failed: {}",
                            worker_id, e
                        );
                        Err(format!("AddressAppend proof failed: {}", e))
                    }
                }
            }
        };

        let result = ProofResult {
            seq: job.seq,
            result: result_data,
        };

        // Send result (success or failure) via the job's own channel
        if job.result_tx.send(result).await.is_err() {
            debug!(
                "ProofWorker {} result channel closed for job seq={}, continuing",
                worker_id, job.seq
            );
        } else {
            debug!("ProofWorker {} completed job seq={}", worker_id, job.seq);
        }
    }

    trace!("ProofWorker {} shutting down", worker_id);
    Ok(())
}
