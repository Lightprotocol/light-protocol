use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use borsh::BorshSerialize;

const MAX_BUFFER_SIZE: usize = 1000;
const V2_IXS_PER_TX: usize = 5;
const MIN_SLOTS_BEFORE_ELIGIBILITY_END: u64 = 2;

use light_batched_merkle_tree::merkle_tree::{
    InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
};
use light_client::rpc::Rpc;
use light_registry::account_compression_cpi::sdk::{
    create_batch_append_instruction, create_batch_nullify_instruction,
    create_batch_update_address_tree_instruction,
};
use solana_sdk::{instruction::Instruction, signature::Signer};
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::{debug, info, warn};

use crate::{
    errors::ForesterError,
    processor::v2::{
        common::send_transaction_batch, proof_cache::SharedProofCache,
        proof_worker::ProofJobResult, BatchContext,
    },
};

#[derive(Debug, Clone, Default)]
pub struct ProofTimings {
    pub append_proof_ms: u64,
    pub nullify_proof_ms: u64,
    pub address_append_proof_ms: u64,
    pub append_round_trip_ms: u64,
    pub nullify_round_trip_ms: u64,
    pub address_append_round_trip_ms: u64,
}

impl ProofTimings {
    pub fn append_proof_duration(&self) -> Duration {
        Duration::from_millis(self.append_proof_ms)
    }
    pub fn nullify_proof_duration(&self) -> Duration {
        Duration::from_millis(self.nullify_proof_ms)
    }
    pub fn address_append_proof_duration(&self) -> Duration {
        Duration::from_millis(self.address_append_proof_ms)
    }
    pub fn append_round_trip_duration(&self) -> Duration {
        Duration::from_millis(self.append_round_trip_ms)
    }
    pub fn nullify_round_trip_duration(&self) -> Duration {
        Duration::from_millis(self.nullify_round_trip_ms)
    }
    pub fn address_append_round_trip_duration(&self) -> Duration {
        Duration::from_millis(self.address_append_round_trip_ms)
    }
}

/// Result of TxSender processing
#[derive(Debug, Clone, Default)]
pub struct TxSenderResult {
    pub items_processed: usize,
    pub proof_timings: ProofTimings,
    /// Number of proofs saved to cache when epoch ended (for potential reuse)
    pub proofs_saved_to_cache: usize,
    /// Total time spent sending transactions
    pub tx_sending_duration: Duration,
}

#[derive(Debug, Clone)]
pub enum BatchInstruction {
    Append(Vec<InstructionDataBatchAppendInputs>),
    Nullify(Vec<InstructionDataBatchNullifyInputs>),
    AddressAppend(Vec<light_batched_merkle_tree::merkle_tree::InstructionDataAddressAppendInputs>),
}

impl BatchInstruction {
    /// Returns the number of ZKP batch instructions contained in this batch.
    pub fn items_count(&self) -> usize {
        match self {
            BatchInstruction::Append(v) => v.len(),
            BatchInstruction::Nullify(v) => v.len(),
            BatchInstruction::AddressAppend(v) => v.len(),
        }
    }
}

/// Entry in the ordered proof buffer: instruction + timing info
#[derive(Clone)]
struct BufferEntry {
    instruction: BatchInstruction,
    round_trip_ms: u64,
    proof_ms: u64,
    submitted_at: std::time::Instant,
}

struct OrderedProofBuffer {
    buffer: Vec<Option<BufferEntry>>,
    base_seq: u64,
    len: usize,
    head: usize,
}

impl OrderedProofBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: (0..capacity).map(|_| None).collect(),
            base_seq: 0,
            len: 0,
            head: 0,
        }
    }

    fn capacity(&self) -> usize {
        self.buffer.len()
    }

    fn len(&self) -> usize {
        self.len
    }

    fn insert(
        &mut self,
        seq: u64,
        instruction: BatchInstruction,
        round_trip_ms: u64,
        proof_ms: u64,
        submitted_at: std::time::Instant,
    ) -> bool {
        if seq < self.base_seq {
            return false;
        }
        let offset = (seq - self.base_seq) as usize;
        if offset >= self.buffer.len() {
            return false;
        }
        let idx = (self.head + offset) % self.buffer.len();
        if self.buffer[idx].is_none() {
            self.len += 1;
        }
        self.buffer[idx] = Some(BufferEntry {
            instruction,
            round_trip_ms,
            proof_ms,
            submitted_at,
        });
        true
    }

    fn pop_next(&mut self) -> Option<BufferEntry> {
        let item = self.buffer[self.head].take();
        if item.is_some() {
            self.len -= 1;
            self.base_seq += 1;
            self.head = (self.head + 1) % self.buffer.len();
        }
        item
    }

    fn expected_seq(&self) -> u64 {
        self.base_seq
    }
}

pub struct TxSender<R: Rpc> {
    context: BatchContext<R>,
    buffer: OrderedProofBuffer,
    zkp_batch_size: u64,
    last_seen_root: [u8; 32],
    pending_batch: Vec<(BatchInstruction, u64)>, // (instruction, seq)
    pending_batch_round_trip_ms: u64,
    pending_batch_proof_ms: u64,
    /// Earliest submission time in the pending batch (for end-to-end latency)
    pending_batch_earliest_submit: Option<std::time::Instant>,
    proof_timings: ProofTimings,
    /// Optional cache to save unused proofs when epoch ends (for reuse in next epoch)
    proof_cache: Option<Arc<SharedProofCache>>,
}

impl<R: Rpc> TxSender<R> {
    pub(crate) fn spawn(
        context: BatchContext<R>,
        proof_rx: mpsc::Receiver<ProofJobResult>,
        zkp_batch_size: u64,
        last_seen_root: [u8; 32],
        proof_cache: Option<Arc<SharedProofCache>>,
    ) -> JoinHandle<crate::Result<TxSenderResult>> {
        let sender = Self {
            context,
            buffer: OrderedProofBuffer::new(MAX_BUFFER_SIZE),
            zkp_batch_size,
            last_seen_root,
            pending_batch: Vec::with_capacity(V2_IXS_PER_TX),
            pending_batch_round_trip_ms: 0,
            pending_batch_proof_ms: 0,
            pending_batch_earliest_submit: None,
            proof_timings: ProofTimings::default(),
            proof_cache,
        };

        tokio::spawn(async move { sender.run(proof_rx).await })
    }

    #[inline]
    fn should_flush_due_to_time_at(&self, current_slot: u64) -> bool {
        let forester_end = self
            .context
            .forester_eligibility_end_slot
            .load(Ordering::Relaxed);
        let eligibility_end_slot = if forester_end > 0 {
            forester_end
        } else {
            self.context.epoch_phases.active.end
        };
        let slots_remaining = eligibility_end_slot.saturating_sub(current_slot);
        slots_remaining < MIN_SLOTS_BEFORE_ELIGIBILITY_END
    }

    #[inline]
    fn is_still_eligible_at(&self, current_slot: u64) -> bool {
        let forester_end = self
            .context
            .forester_eligibility_end_slot
            .load(Ordering::Relaxed);
        let eligibility_end_slot = if forester_end > 0 {
            forester_end
        } else {
            self.context.epoch_phases.active.end
        };
        current_slot + MIN_SLOTS_BEFORE_ELIGIBILITY_END < eligibility_end_slot
    }

    async fn run(
        mut self,
        mut proof_rx: mpsc::Receiver<ProofJobResult>,
    ) -> crate::Result<TxSenderResult> {
        let (batch_tx, mut batch_rx) = mpsc::unbounded_channel::<(
            Vec<(BatchInstruction, u64)>,
            u64,
            u64,
            Option<std::time::Instant>,
        )>();

        let sender_context = self.context.clone();
        let mut sender_last_root = self.last_seen_root;
        let zkp_batch_size_val = self.zkp_batch_size;

        let sender_handle = tokio::spawn(async move {
            let mut sender_processed = 0usize;
            let mut total_tx_sending_duration = Duration::ZERO;
            while let Some((batch, batch_len, _batch_round_trip, batch_earliest_submit)) =
                batch_rx.recv().await
            {
                let items_count = batch.len();
                let first_seq = batch.first().map(|(_, s)| *s).unwrap_or(0);
                let last_seq = batch.last().map(|(_, s)| *s).unwrap_or(0);

                let mut all_instructions: Vec<Instruction> = Vec::new();
                let mut last_root: Option<[u8; 32]> = None;
                let mut append_count = 0usize;
                let mut nullify_count = 0usize;
                let mut _address_append_count = 0usize;

                for (instr, _seq) in &batch {
                    let res = match instr {
                        BatchInstruction::Append(proofs) => {
                            append_count += 1;
                            let ix_res = proofs
                                .iter()
                                .map(|data| {
                                    Ok(create_batch_append_instruction(
                                        sender_context.authority.pubkey(),
                                        sender_context.derivation,
                                        sender_context.merkle_tree,
                                        sender_context.output_queue,
                                        sender_context.epoch,
                                        data.try_to_vec()?,
                                    ))
                                })
                                .collect::<anyhow::Result<Vec<_>>>()?;
                            (ix_res, proofs.last().map(|p| p.new_root))
                        }
                        BatchInstruction::Nullify(proofs) => {
                            nullify_count += 1;
                            let ix_res = proofs
                                .iter()
                                .map(|data| {
                                    Ok(create_batch_nullify_instruction(
                                        sender_context.authority.pubkey(),
                                        sender_context.derivation,
                                        sender_context.merkle_tree,
                                        sender_context.epoch,
                                        data.try_to_vec()?,
                                    ))
                                })
                                .collect::<anyhow::Result<Vec<_>>>()?;
                            (ix_res, proofs.last().map(|p| p.new_root))
                        }
                        BatchInstruction::AddressAppend(proofs) => {
                            _address_append_count += 1;
                            let ix_res = proofs
                                .iter()
                                .map(|data| {
                                    Ok(create_batch_update_address_tree_instruction(
                                        sender_context.authority.pubkey(),
                                        sender_context.derivation,
                                        sender_context.merkle_tree,
                                        sender_context.epoch,
                                        data.try_to_vec()?,
                                    ))
                                })
                                .collect::<anyhow::Result<Vec<_>>>()?;
                            (ix_res, proofs.last().map(|p| p.new_root))
                        }
                    };
                    all_instructions.extend(res.0);
                    if let Some(root) = res.1 {
                        last_root = Some(root);
                    }
                }

                let instr_type = if append_count > 0 && nullify_count > 0 {
                    format!("Append+Nullify({}+{})", append_count, nullify_count)
                } else if append_count > 0 {
                    "Append".to_string()
                } else if nullify_count > 0 {
                    "Nullify".to_string()
                } else {
                    "AddressAppend".to_string()
                };

                let send_start = std::time::Instant::now();
                match send_transaction_batch(&sender_context, all_instructions).await {
                    Ok(sig) => {
                        total_tx_sending_duration += send_start.elapsed();
                        if let Some(root) = last_root {
                            sender_last_root = root;
                        }
                        let items_processed = batch_len as usize * zkp_batch_size_val as usize;
                        sender_processed += items_processed;
                        let e2e_ms = batch_earliest_submit
                            .map(|t| t.elapsed().as_millis() as u64)
                            .unwrap_or(0);
                        info!(
                            "tx sent: {} type={} ixs={} tree={} root={:?} seq={}..{} epoch={} e2e={}ms",
                            sig,
                            instr_type,
                            items_count,
                            sender_context.merkle_tree,
                            &sender_last_root[..4],
                            first_seq,
                            last_seq,
                            sender_context.epoch,
                            e2e_ms,
                        );
                    }
                    Err(e) => {
                        total_tx_sending_duration += send_start.elapsed();
                        warn!("tx error {} epoch {}", e, sender_context.epoch);
                        if let Some(ForesterError::NotInActivePhase) =
                            e.downcast_ref::<ForesterError>()
                        {
                            warn!("Active phase ended while sending tx, stopping sender loop");
                            return Ok::<_, anyhow::Error>((sender_processed, total_tx_sending_duration));
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
            Ok((sender_processed, total_tx_sending_duration))
        });

        loop {
            if sender_handle.is_finished() {
                break;
            }

            let result = match proof_rx.recv().await {
                Some(r) => r,
                None => break,
            };

            let current_slot = self.context.slot_tracker.estimated_current_slot();

            if !self.is_still_eligible_at(current_slot) {
                let proofs_saved = self.save_proofs_to_cache(&mut proof_rx, Some(result)).await;
                info!(
                    "Active phase ended for epoch {}, stopping tx sender (saved {} proofs to cache)",
                    self.context.epoch, proofs_saved
                );
                drop(batch_tx);
                let (items_processed, tx_sending_duration) = sender_handle
                    .await
                    .map_err(|e| anyhow::anyhow!("Sender panic: {}", e))??;
                return Ok(TxSenderResult {
                    items_processed,
                    proof_timings: self.proof_timings,
                    proofs_saved_to_cache: proofs_saved,
                    tx_sending_duration,
                });
            }

            if let Ok(instr) = &result.result {
                match instr {
                    BatchInstruction::Append(_) => {
                        self.proof_timings.append_proof_ms += result.proof_duration_ms;
                        self.proof_timings.append_round_trip_ms += result.round_trip_ms;
                    }
                    BatchInstruction::Nullify(_) => {
                        self.proof_timings.nullify_proof_ms += result.proof_duration_ms;
                        self.proof_timings.nullify_round_trip_ms += result.round_trip_ms;
                    }
                    BatchInstruction::AddressAppend(_) => {
                        self.proof_timings.address_append_proof_ms += result.proof_duration_ms;
                        self.proof_timings.address_append_round_trip_ms += result.round_trip_ms;
                    }
                }
            }

            let instruction = match result.result {
                Ok(instr) => instr,
                Err(e) => {
                    warn!("Proof failed seq={}: {}", result.seq, e);
                    return Err(anyhow::anyhow!("Proof failed seq={}: {}", result.seq, e));
                }
            };

            if self.buffer.len() >= self.buffer.capacity() {
                return Err(anyhow::anyhow!("Proof buffer overflow"));
            }
            if !self.buffer.insert(
                result.seq,
                instruction,
                result.round_trip_ms,
                result.proof_duration_ms,
                result.submitted_at,
            ) {
                warn!("Failed to insert proof seq={}", result.seq);
            }

            while let Some(entry) = self.buffer.pop_next() {
                let seq = self.buffer.expected_seq() - 1;
                self.pending_batch.push((entry.instruction, seq));
                self.pending_batch_round_trip_ms += entry.round_trip_ms;
                self.pending_batch_proof_ms += entry.proof_ms;
                self.pending_batch_earliest_submit =
                    Some(match self.pending_batch_earliest_submit {
                        None => entry.submitted_at,
                        Some(existing) => existing.min(entry.submitted_at),
                    });

                let should_send = self.pending_batch.len() >= V2_IXS_PER_TX
                    || (!self.pending_batch.is_empty()
                        && self.should_flush_due_to_time_at(current_slot));

                if should_send {
                    let batch = std::mem::replace(
                        &mut self.pending_batch,
                        Vec::with_capacity(V2_IXS_PER_TX),
                    );
                    let round_trip = std::mem::replace(&mut self.pending_batch_round_trip_ms, 0);
                    let _proof_ms = std::mem::replace(&mut self.pending_batch_proof_ms, 0);
                    let earliest = self.pending_batch_earliest_submit.take();
                    let batch_len = batch.len() as u64;

                    if batch_tx
                        .send((batch, batch_len, round_trip, earliest))
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }

        if !self.pending_batch.is_empty() {
            let batch =
                std::mem::replace(&mut self.pending_batch, Vec::with_capacity(V2_IXS_PER_TX));
            let round_trip = std::mem::replace(&mut self.pending_batch_round_trip_ms, 0);
            let earliest = self.pending_batch_earliest_submit.take();
            let batch_len = batch.len() as u64;
            let _ = batch_tx.send((batch, batch_len, round_trip, earliest));
        }

        drop(batch_tx);
        let (items_processed, tx_sending_duration) = sender_handle
            .await
            .map_err(|e| anyhow::anyhow!("Sender panic: {}", e))??;

        Ok(TxSenderResult {
            items_processed,
            proof_timings: self.proof_timings,
            proofs_saved_to_cache: 0,
            tx_sending_duration,
        })
    }

    async fn save_proofs_to_cache(
        &self,
        proof_rx: &mut mpsc::Receiver<ProofJobResult>,
        current_result: Option<ProofJobResult>,
    ) -> usize {
        let cache = match &self.proof_cache {
            Some(c) => c,
            None => {
                debug!("No proof cache available, discarding remaining proofs");
                return 0;
            }
        };

        let mut saved = 0;

        cache.start_warming(self.last_seen_root).await;

        if let Some(result) = current_result {
            if let Ok(instruction) = result.result {
                cache
                    .add_proof(result.seq, result.old_root, result.new_root, instruction)
                    .await;
                saved += 1;
            }
        }

        while let Ok(result) = proof_rx.try_recv() {
            if let Ok(instruction) = result.result {
                cache
                    .add_proof(result.seq, result.old_root, result.new_root, instruction)
                    .await;
                saved += 1;
            }
        }

        cache.finish_warming().await;

        if saved > 0 {
            info!(
                "Saved {} proofs to cache for potential reuse (root: {:?})",
                saved,
                &self.last_seen_root[..4]
            );
        }

        saved
    }
}
