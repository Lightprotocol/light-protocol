use std::time::Duration;

use anyhow::anyhow;
use async_channel::Receiver;
use light_batched_merkle_tree::merkle_tree::{
    InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
};
use light_prover_client::{
    proof_client::ProofClient,
    proof_types::{
        batch_append::BatchAppendsCircuitInputs, batch_update::BatchUpdateCircuitInputs,
    },
};
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::{debug, info, trace, warn};

use crate::processor::v2::state::tx_sender::BatchInstruction;

#[derive(Debug)]
pub enum ProofInput {
    Append(BatchAppendsCircuitInputs),
    Nullify(BatchUpdateCircuitInputs),
}

#[derive(Debug)]
pub struct ProofJob {
    pub(crate) seq: u64,
    pub(crate) inputs: ProofInput,
}

#[derive(Debug)]
pub struct ProofResult {
    pub(crate) seq: u64,
    pub(crate) instruction: BatchInstruction,
}

#[derive(Clone)]
struct ProverConfig {
    append_url: String,
    update_url: String,
    api_key: Option<String>,
    polling_interval: Duration,
    max_wait_time: Duration,
}

/// Spawns N proof workers that pull jobs from a shared async_channel.
/// Returns the job sender and worker handles.
pub fn spawn_proof_workers(
    num_workers: usize,
    prover_append_url: String,
    prover_update_url: String,
    prover_api_key: Option<String>,
    polling_interval: Duration,
    max_wait_time: Duration,
    result_tx: mpsc::Sender<ProofResult>,
) -> (
    async_channel::Sender<ProofJob>,
    Vec<JoinHandle<crate::Result<()>>>,
) {
    let (job_tx, job_rx) = async_channel::unbounded::<ProofJob>();

    let config = ProverConfig {
        append_url: prover_append_url,
        update_url: prover_update_url,
        api_key: prover_api_key,
        polling_interval,
        max_wait_time,
    };

    let mut handles = Vec::with_capacity(num_workers);

    for worker_id in 0..num_workers {
        let job_rx = job_rx.clone();
        let result_tx = result_tx.clone();
        let config = config.clone();

        let handle =
            tokio::spawn(
                async move { run_proof_worker(worker_id, job_rx, result_tx, config).await },
            );

        handles.push(handle);
    }

    info!("Spawned {} proof workers", num_workers);
    (job_tx, handles)
}

async fn run_proof_worker(
    worker_id: usize,
    job_rx: Receiver<ProofJob>,
    result_tx: mpsc::Sender<ProofResult>,
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

        let result = match job.inputs {
            ProofInput::Append(inputs) => {
                let (proof, new_root) = append_client
                    .generate_batch_append_proof(inputs)
                    .await
                    .map_err(|e| anyhow!("ProofWorker {} append proof failed: {}", worker_id, e))?;

                ProofResult {
                    seq: job.seq,
                    instruction: BatchInstruction::Append(vec![InstructionDataBatchAppendInputs {
                        new_root,
                        compressed_proof:
                            light_compressed_account::instruction_data::compressed_proof::CompressedProof {
                                a: proof.a,
                                b: proof.b,
                                c: proof.c,
                            },
                    }]),
                }
            }
            ProofInput::Nullify(inputs) => {
                let (proof, new_root) = nullify_client
                    .generate_batch_update_proof(inputs)
                    .await
                    .map_err(|e| {
                        anyhow!("ProofWorker {} nullify proof failed: {}", worker_id, e)
                    })?;

                ProofResult {
                    seq: job.seq,
                    instruction: BatchInstruction::Nullify(vec![InstructionDataBatchNullifyInputs {
                        new_root,
                        compressed_proof:
                            light_compressed_account::instruction_data::compressed_proof::CompressedProof {
                                a: proof.a,
                                b: proof.b,
                                c: proof.c,
                            },
                    }]),
                }
            }
        };

        if result_tx.send(result).await.is_err() {
            warn!("ProofWorker {} result channel closed", worker_id);
            break;
        }

        debug!("ProofWorker {} completed job seq={}", worker_id, job.seq);
    }

    trace!("ProofWorker {} shutting down", worker_id);
    Ok(())
}
