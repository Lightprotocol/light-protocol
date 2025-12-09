use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use borsh::BorshSerialize;

// Maximum number of buffered proof results
const MAX_BUFFER_SIZE: usize = 1000;

// Number of proof instructions to bundle per transaction
pub const V2_IXS_PER_TX: usize = 4;

// Minimum slots remaining before we force-send any pending batch
const MIN_SLOTS_FOR_BATCHING: u64 = 10;

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
        common::send_transaction_batch, proof_cache::SharedProofCache, proof_worker::ProofResult,
        BatchContext,
    },
};

/// Aggregated proof times by circuit type
#[derive(Debug, Clone, Default)]
pub struct ProofTimings {
    /// Pure proof generation times from prover server (excludes queue wait)
    pub append_proof_ms: u64,
    pub nullify_proof_ms: u64,
    pub address_append_proof_ms: u64,
    /// Round-trip times (submit to result, includes queue wait + proof)
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
}

#[derive(Debug, Clone)]
pub enum BatchInstruction {
    Append(Vec<InstructionDataBatchAppendInputs>),
    Nullify(Vec<InstructionDataBatchNullifyInputs>),
    AddressAppend(Vec<light_batched_merkle_tree::merkle_tree::InstructionDataAddressAppendInputs>),
}

struct OrderedProofBuffer {
    buffer: Vec<Option<BatchInstruction>>,
    base_seq: u64,
    len: usize,
}

impl OrderedProofBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: (0..capacity).map(|_| None).collect(),
            base_seq: 0,
            len: 0,
        }
    }

    fn capacity(&self) -> usize {
        self.buffer.len()
    }

    fn len(&self) -> usize {
        self.len
    }

    fn insert(&mut self, seq: u64, instruction: BatchInstruction) -> bool {
        if seq < self.base_seq {
            return false;
        }
        let offset = (seq - self.base_seq) as usize;
        if offset >= self.buffer.len() {
            return false;
        }
        if self.buffer[offset].is_none() {
            self.len += 1;
        }
        self.buffer[offset] = Some(instruction);
        true
    }

    fn pop_next(&mut self) -> Option<BatchInstruction> {
        let item = self.buffer[0].take();
        if item.is_some() {
            self.len -= 1;
            self.base_seq += 1;
            self.buffer.rotate_left(1);
        }
        item
    }

    fn expected_seq(&self) -> u64 {
        self.base_seq
    }

    fn oldest_buffered(&self) -> Option<u64> {
        for (i, item) in self.buffer.iter().enumerate() {
            if item.is_some() {
                return Some(self.base_seq + i as u64);
            }
        }
        None
    }
}

pub struct TxSender<R: Rpc> {
    context: BatchContext<R>,
    buffer: OrderedProofBuffer,
    zkp_batch_size: u64,
    last_seen_root: [u8; 32],
    pending_batch: Vec<(BatchInstruction, u64)>, // (instruction, seq)
    proof_timings: ProofTimings,
    /// Optional cache to save unused proofs when epoch ends (for reuse in next epoch)
    proof_cache: Option<Arc<SharedProofCache>>,
}

impl<R: Rpc> TxSender<R> {
    pub(crate) fn spawn(
        context: BatchContext<R>,
        proof_rx: mpsc::Receiver<ProofResult>,
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
        slots_remaining < MIN_SLOTS_FOR_BATCHING
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
        // Stop 2 slots before eligibility ends to avoid race conditions
        current_slot + 2 < eligibility_end_slot
    }

    async fn run(mut self, mut proof_rx: mpsc::Receiver<ProofResult>) -> crate::Result<TxSenderResult> {
        let mut processed = 0usize;

        while let Some(result) = proof_rx.recv().await {
            let current_slot = self.context.slot_tracker.estimated_current_slot();

            if !self.is_still_eligible_at(current_slot) {
                // Save current proof and any remaining proofs to cache for potential reuse
                let proofs_saved = self
                    .save_proofs_to_cache(&mut proof_rx, Some(result))
                    .await;
                info!(
                    "Active phase ended for epoch {}, stopping tx sender (saved {} proofs to cache)",
                    self.context.epoch, proofs_saved
                );
                return Ok(TxSenderResult {
                    items_processed: processed,
                    proof_timings: self.proof_timings,
                    proofs_saved_to_cache: proofs_saved,
                });
            }

            // Track proof times by circuit type
            match &result.result {
                Ok(instr) => {
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
                Err(_) => {} // Don't track failed proofs
            }

            // Handle proof failures
            let instruction = match result.result {
                Ok(instr) => instr,
                Err(e) => {
                    warn!(
                        "Proof generation failed for seq={}: {}. Stopping batch processing.",
                        result.seq, e
                    );
                    return Err(anyhow::anyhow!(
                        "Proof generation failed for batch {}: {}",
                        result.seq,
                        e
                    ));
                }
            };

            if self.buffer.len() >= self.buffer.capacity() {
                warn!(
                    "Buffer overflow: {} buffered proofs (max {}). Expected seq={}, oldest buffered={:?}",
                    self.buffer.len(),
                    self.buffer.capacity(),
                    self.buffer.expected_seq(),
                    self.buffer.oldest_buffered()
                );
                return Err(anyhow::anyhow!(
                    "Proof buffer overflow: possible missing proof for seq={}",
                    self.buffer.expected_seq()
                ));
            }

            if !self.buffer.insert(result.seq, instruction) {
                warn!(
                    "Failed to insert proof seq={} (base={}, capacity={})",
                    result.seq,
                    self.buffer.expected_seq(),
                    self.buffer.capacity()
                );
            }

            while let Some(instr) = self.buffer.pop_next() {
                let seq = self.buffer.expected_seq() - 1; // pop_next already incremented
                self.pending_batch.push((instr, seq));

                // Send batch when:
                // 1. We have enough instructions, OR
                // 2. We're running low on time (epoch ending soon)
                let should_send = self.pending_batch.len() >= V2_IXS_PER_TX
                    || (!self.pending_batch.is_empty()
                        && self.should_flush_due_to_time_at(current_slot));

                if should_send {
                    processed += self.send_pending_batch().await?;
                }
            }
        }

        if !self.pending_batch.is_empty() {
            processed += self.send_pending_batch().await?;
        }

        Ok(TxSenderResult {
            items_processed: processed,
            proof_timings: self.proof_timings,
            proofs_saved_to_cache: 0,
        })
    }

    /// Save remaining proofs to cache when epoch ends, for potential reuse in next epoch.
    /// Returns the number of proofs saved.
    async fn save_proofs_to_cache(
        &self,
        proof_rx: &mut mpsc::Receiver<ProofResult>,
        current_result: Option<ProofResult>,
    ) -> usize {
        let cache = match &self.proof_cache {
            Some(c) => c,
            None => {
                debug!("No proof cache available, discarding remaining proofs");
                return 0;
            }
        };

        let mut saved = 0;

        // Start warming the cache with the current root
        cache.start_warming(self.last_seen_root).await;

        // Save current result if present
        if let Some(result) = current_result {
            if let Ok(instruction) = result.result {
                cache
                    .add_proof(result.seq, result.old_root, result.new_root, instruction)
                    .await;
                saved += 1;
            }
        }

        // Drain remaining proofs from channel
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

    async fn send_pending_batch(&mut self) -> crate::Result<usize> {
        if self.pending_batch.is_empty() {
            return Ok(0);
        }

        let batch = std::mem::replace(&mut self.pending_batch, Vec::with_capacity(V2_IXS_PER_TX));

        let batch_len = batch.len();
        let first_seq = batch.first().map(|(_, s)| *s).unwrap_or(0);
        let last_seq = batch.last().map(|(_, s)| *s).unwrap_or(0);

        let mut all_instructions: Vec<Instruction> = Vec::new();
        let mut last_root: Option<[u8; 32]> = None;
        let mut append_count = 0usize;
        let mut nullify_count = 0usize;
        let mut _address_append_count = 0usize;

        for (instr, _seq) in &batch {
            let (instructions, expected_root) = match instr {
                BatchInstruction::Append(proofs) => {
                    append_count += 1;
                    let ix = proofs
                        .iter()
                        .map(|data| {
                            Ok(create_batch_append_instruction(
                                self.context.authority.pubkey(),
                                self.context.derivation,
                                self.context.merkle_tree,
                                self.context.output_queue,
                                self.context.epoch,
                                data.try_to_vec()?,
                            ))
                        })
                        .collect::<anyhow::Result<Vec<_>>>()?;
                    (ix, proofs.last().map(|p| p.new_root))
                }
                BatchInstruction::Nullify(proofs) => {
                    nullify_count += 1;
                    let ix = proofs
                        .iter()
                        .map(|data| {
                            Ok(create_batch_nullify_instruction(
                                self.context.authority.pubkey(),
                                self.context.derivation,
                                self.context.merkle_tree,
                                self.context.epoch,
                                data.try_to_vec()?,
                            ))
                        })
                        .collect::<anyhow::Result<Vec<_>>>()?;
                    (ix, proofs.last().map(|p| p.new_root))
                }
                BatchInstruction::AddressAppend(proofs) => {
                    _address_append_count += 1;
                    let ix = proofs
                        .iter()
                        .map(|data| {
                            Ok(create_batch_update_address_tree_instruction(
                                self.context.authority.pubkey(),
                                self.context.derivation,
                                self.context.merkle_tree,
                                self.context.epoch,
                                data.try_to_vec()?,
                            ))
                        })
                        .collect::<anyhow::Result<Vec<_>>>()?;
                    (ix, proofs.last().map(|p| p.new_root))
                }
            };

            all_instructions.extend(instructions);
            if let Some(root) = expected_root {
                last_root = Some(root);
            }
        }

        // Build instruction type string for logging
        let instr_type = if append_count > 0 && nullify_count > 0 {
            format!("Append+Nullify({}+{})", append_count, nullify_count)
        } else if append_count > 0 {
            "Append".to_string()
        } else if nullify_count > 0 {
            "Nullify".to_string()
        } else {
            "AddressAppend".to_string()
        };

        match send_transaction_batch(&self.context, all_instructions).await {
            Ok(sig) => {
                if let Some(root) = last_root {
                    self.last_seen_root = root;
                }
                let items_processed = batch_len * self.zkp_batch_size as usize;
                info!(
                    "tx sent: {} type={} ixs={} root={:?} seq={}..{} epoch={}",
                    sig,
                    instr_type,
                    batch_len,
                    &self.last_seen_root[..4],
                    first_seq,
                    last_seq,
                    self.context.epoch
                );
                Ok(items_processed)
            }
            Err(e) => {
                warn!("tx error {} epoch {}", e, self.context.epoch);
                if let Some(ForesterError::NotInActivePhase) = e.downcast_ref::<ForesterError>() {
                    warn!("Active phase ended while sending tx, stopping sender loop");
                    Ok(0)
                } else {
                    Err(e)
                }
            }
        }
    }
}
