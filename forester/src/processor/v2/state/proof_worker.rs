use std::time::Duration;
use anyhow::anyhow;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use forester_utils::instructions::state::BatchInstruction;
use light_batched_merkle_tree::merkle_tree::{InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs};
use light_client::rpc::Rpc;
use light_prover_client::proof_client::ProofClient;
use light_prover_client::proof_types::batch_append::BatchAppendsCircuitInputs;
use light_prover_client::proof_types::batch_update::BatchUpdateCircuitInputs;
use crate::processor::v2::BatchContext;

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

pub fn spawn_proof_workers<R: Rpc>(
    context: &BatchContext<R>,
    mut job_rx: mpsc::Receiver<ProofJob>,
    proof_tx: mpsc::Sender<ProofResult>,
    polling_interval: Duration,
    max_wait_time: Duration,
) -> Vec<JoinHandle<crate::Result<()>>> {
    let append_client = ProofClient::with_config(
        context.prover_append_url.clone(),
        polling_interval,
        max_wait_time,
        context.prover_api_key.clone(),
    );
    let nullify_client = ProofClient::with_config(
        context.prover_update_url.clone(),
        polling_interval,
        max_wait_time,
        context.prover_api_key.clone(),
    );
    let proof_tx_clone = proof_tx.clone();

    let handle = tokio::spawn(async move {
        while let Some(job) = job_rx.recv().await {
            match job.inputs {
                ProofInput::Append(inputs) => {
                    let (proof, new_root) = append_client
                        .generate_batch_append_proof(inputs)
                        .await
                        .map_err(|e| anyhow!("ACTOR Append proof generation failed: {}", e))?;
                    let instruction = InstructionDataBatchAppendInputs {
                        new_root,
                        compressed_proof: light_compressed_account::instruction_data::compressed_proof::CompressedProof { a: proof.a, b: proof.b, c: proof.c },
                    };
                    proof_tx_clone
                        .send(ProofResult {
                            seq: job.seq,
                            instruction: BatchInstruction::Append(vec![instruction]),
                        })
                        .await?;
                }
                ProofInput::Nullify(inputs) => {
                    let (proof, new_root) = nullify_client
                        .generate_batch_update_proof(inputs)
                        .await
                        .map_err(|e| anyhow!("ACTOR Nullify proof generation failed: {}", e))?;
                    let instruction = InstructionDataBatchNullifyInputs {
                        new_root,
                        compressed_proof: light_compressed_account::instruction_data::compressed_proof::CompressedProof { a: proof.a, b: proof.b, c: proof.c },
                    };
                    proof_tx_clone
                        .send(ProofResult {
                            seq: job.seq,
                            instruction: BatchInstruction::Nullify(vec![instruction]),
                        })
                        .await?;
                }
            }
        }
        Ok(())
    });

    vec![handle]
}
