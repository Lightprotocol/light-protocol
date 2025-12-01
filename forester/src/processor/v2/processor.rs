use std::{sync::atomic::Ordering, time::Instant};

use crate::{
    epoch_manager::{CircuitMetrics, ProcessingMetrics},
    processor::v2::{
        common::WorkerPool,
        proof_worker::{spawn_proof_workers, ProofInput, ProofJob, ProofResult},
        strategy::{CircuitType, QueueData, TreeStrategy},
        tx_sender::TxSender,
        BatchContext, ProcessingResult, QueueWork,
    },
};
use anyhow::anyhow;
use light_client::rpc::Rpc;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::mpsc;
use tracing::{debug, warn};

const MAX_BATCHES_LIMIT: usize = 1000;

#[derive(Debug)]
pub struct QueueProcessor<R: Rpc, S: TreeStrategy<R>> {
    context: BatchContext<R>,
    strategy: S,
    current_root: [u8; 32],
    zkp_batch_size: u64,
    seq: u64,
    worker_pool: Option<WorkerPool>,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: Rpc, S: TreeStrategy<R>> QueueProcessor<R, S> {
    pub async fn new(context: BatchContext<R>, strategy: S) -> crate::Result<Self> {
        let zkp_batch_size = strategy.fetch_zkp_batch_size(&context).await?;
        Ok(Self {
            context,
            strategy,
            current_root: [0u8; 32],
            zkp_batch_size,
            seq: 0,
            worker_pool: None,
            _phantom: std::marker::PhantomData,
        })
    }

    pub async fn process_queue_update(
        &mut self,
        queue_work: QueueWork,
    ) -> crate::Result<ProcessingResult> {
        if queue_work.queue_size < self.zkp_batch_size {
            return Ok(ProcessingResult::default());
        }

        let max_batches =
            ((queue_work.queue_size / self.zkp_batch_size) as usize).min(MAX_BATCHES_LIMIT);

        if queue_work.queue_size / self.zkp_batch_size > MAX_BATCHES_LIMIT as u64 {
            debug!(
                "Queue size {} would produce {} batches, limiting to {}",
                queue_work.queue_size,
                queue_work.queue_size / self.zkp_batch_size,
                MAX_BATCHES_LIMIT
            );
        }

        if self.worker_pool.is_none() {
            let num_workers = self.context.num_proof_workers.max(1);
            let job_tx = spawn_proof_workers(num_workers, &self.context.prover_config);
            self.worker_pool = Some(WorkerPool { job_tx });
        }

        let queue_data = match self
            .strategy
            .fetch_queue_data(&self.context, &queue_work, max_batches, self.zkp_batch_size)
            .await?
        {
            Some(data) => data,
            None => return Ok(ProcessingResult::default()),
        };

        self.process_batches(queue_data).await
    }

    pub fn update_eligibility(&mut self, end_slot: u64) {
        self.context
            .forester_eligibility_end_slot
            .store(end_slot, Ordering::Relaxed);
    }

    pub fn merkle_tree(&self) -> &Pubkey {
        &self.context.merkle_tree
    }

    pub fn epoch(&self) -> u64 {
        self.context.epoch
    }
    async fn process_batches(
        &mut self,
        mut queue_data: QueueData<S::StagingTree>,
    ) -> crate::Result<ProcessingResult> {
        self.current_root = queue_data.initial_root;
        let num_batches = queue_data.num_batches;
        let num_workers = self.context.num_proof_workers.max(1);
        let (proof_tx, proof_rx) = mpsc::channel(num_workers * 2);

        self.seq = 0;
        let proof_gen_start = Instant::now();
        let tx_sender_handle = TxSender::spawn(
            self.context.clone(),
            proof_rx,
            self.zkp_batch_size,
            self.current_root,
        );
        let job_tx = self
            .worker_pool
            .as_ref()
            .ok_or_else(|| anyhow!("Worker pool not initialized"))?
            .job_tx
            .clone();

        let circuit_inputs_start = Instant::now();
        let jobs_sent = self
            .enqueue_jobs(&mut queue_data, num_batches, job_tx, proof_tx.clone())
            .await?;
        let circuit_inputs_duration = circuit_inputs_start.elapsed();

        drop(proof_tx);

        let tx_processed = tx_sender_handle
            .await
            .map_err(|e| anyhow!("Tx sender join error: {}", e))
            .and_then(|res| res)
            .inspect_err(|e| {
                warn!(
                    "Tx sender error for tree {}: {}",
                    self.context.merkle_tree, e
                );
            })?;

        let total_proof_and_tx_time = proof_gen_start.elapsed();

        let proof_generation_duration =
            total_proof_and_tx_time.saturating_sub(circuit_inputs_duration);

        if tx_processed < jobs_sent * self.zkp_batch_size as usize {
            debug!(
                "Processed {} items but expected {}, some proofs may have failed",
                tx_processed,
                jobs_sent * self.zkp_batch_size as usize
            );
        }

        let circuit_type = self.strategy.circuit_type(&queue_data.staging_tree);
        let circuit_metrics = CircuitMetrics {
            circuit_inputs_duration,
            proof_generation_duration,
        };

        let mut metrics = ProcessingMetrics::default();
        match circuit_type {
            CircuitType::Append => metrics.append = circuit_metrics,
            CircuitType::Nullify => metrics.nullify = circuit_metrics,
            CircuitType::AddressAppend => metrics.address_append = circuit_metrics,
        }

        Ok(ProcessingResult {
            items_processed: tx_processed,
            metrics,
        })
    }

    async fn enqueue_jobs(
        &mut self,
        queue_data: &mut QueueData<S::StagingTree>,
        num_batches: usize,
        job_tx: async_channel::Sender<ProofJob>,
        result_tx: mpsc::Sender<ProofResult>,
    ) -> crate::Result<usize> {
        let zkp_batch_size = self.zkp_batch_size as usize;
        let mut jobs_sent = 0;

        for batch_idx in 0..num_batches {
            let start = batch_idx * zkp_batch_size;

            let (inputs, new_root) = self.strategy.build_proof_job(
                &mut queue_data.staging_tree,
                batch_idx,
                start,
                self.zkp_batch_size,
            )?;

            let job = self.finish_job(new_root, inputs, result_tx.clone());
            job_tx.send(job).await?;
            jobs_sent += 1;
        }

        Ok(jobs_sent)
    }

    fn finish_job(
        &mut self,
        new_root: [u8; 32],
        inputs: ProofInput,
        result_tx: mpsc::Sender<ProofResult>,
    ) -> ProofJob {
        self.current_root = new_root;
        let job_seq = self.seq;
        self.seq += 1;
        ProofJob {
            seq: job_seq,
            inputs,
            result_tx,
        }
    }
}
