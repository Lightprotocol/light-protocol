use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

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

use crate::processor::v2::{state::tx_sender::BatchInstruction, ProverConfig};

#[derive(Clone, Default)]
pub struct CancellationFlag(Arc<AtomicBool>);

impl CancellationFlag {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

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

pub fn spawn_proof_workers(
    num_workers: usize,
    config: ProverConfig,
    result_tx: mpsc::Sender<ProofResult>,
) -> (
    async_channel::Sender<ProofJob>,
    CancellationFlag,
    Vec<JoinHandle<crate::Result<()>>>,
) {
    // Enforce minimum of 1 worker to prevent zero-capacity channels and no workers
    let num_workers = if num_workers == 0 {
        warn!("spawn_proof_workers called with num_workers=0, using 1 instead");
        1
    } else {
        num_workers
    };

    let channel_capacity = num_workers * 2;
    let (job_tx, job_rx) = async_channel::bounded::<ProofJob>(channel_capacity);
    let cancel_flag = CancellationFlag::new();

    let mut handles = Vec::with_capacity(num_workers);

    for worker_id in 0..num_workers {
        let job_rx = job_rx.clone();
        let result_tx = result_tx.clone();
        let config = config.clone();
        let cancel_flag = cancel_flag.clone();

        let handle = tokio::spawn(async move {
            run_proof_worker(worker_id, job_rx, result_tx, config, cancel_flag).await
        });

        handles.push(handle);
    }

    info!("Spawned {} proof workers", num_workers);
    (job_tx, cancel_flag, handles)
}

async fn run_proof_worker(
    worker_id: usize,
    job_rx: Receiver<ProofJob>,
    result_tx: mpsc::Sender<ProofResult>,
    config: ProverConfig,
    cancel_flag: CancellationFlag,
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
        if cancel_flag.is_cancelled() {
            debug!(
                "ProofWorker {} stopping due to cancellation (before job seq={})",
                worker_id, job.seq
            );
            break;
        }

        debug!("ProofWorker {} processing job seq={}", worker_id, job.seq);

        let result = match job.inputs {
            ProofInput::Append(inputs) => {
                match append_client.generate_batch_append_proof(inputs).await {
                    Ok((proof, new_root)) => {
                        if cancel_flag.is_cancelled() {
                            debug!(
                                "ProofWorker {} stopping due to cancellation (after job seq={})",
                                worker_id, job.seq
                            );
                            break;
                        }
                        ProofResult {
                            seq: job.seq,
                            instruction: BatchInstruction::Append(vec![
                                InstructionDataBatchAppendInputs {
                                    new_root,
                                    compressed_proof: proof.into(),
                                },
                            ]),
                        }
                    }
                    Err(e) => {
                        cancel_flag.cancel();
                        warn!(
                            "ProofWorker {} append proof failed, cancelling all workers: {}",
                            worker_id, e
                        );
                        return Err(anyhow!(
                            "ProofWorker {} append proof failed: {}",
                            worker_id,
                            e
                        ));
                    }
                }
            }
            ProofInput::Nullify(inputs) => {
                match nullify_client.generate_batch_update_proof(inputs).await {
                    Ok((proof, new_root)) => {
                        if cancel_flag.is_cancelled() {
                            debug!(
                                "ProofWorker {} stopping due to cancellation (after job seq={})",
                                worker_id, job.seq
                            );
                            break;
                        }
                        ProofResult {
                            seq: job.seq,
                            instruction: BatchInstruction::Nullify(vec![
                                InstructionDataBatchNullifyInputs {
                                    new_root,
                                    compressed_proof: proof.into(),
                                },
                            ]),
                        }
                    }
                    Err(e) => {
                        cancel_flag.cancel();
                        warn!(
                            "ProofWorker {} nullify proof failed, cancelling all workers: {}",
                            worker_id, e
                        );
                        return Err(anyhow!(
                            "ProofWorker {} nullify proof failed: {}",
                            worker_id,
                            e
                        ));
                    }
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
