use std::{sync::Arc, time::Duration};

use borsh::BorshSerialize;

const MAX_BUFFER_SIZE: usize = 1000;
const V2_IXS_PER_TX_WITH_LUT: usize = 5;
const V2_IXS_PER_TX_WITHOUT_LUT: usize = 4;
/// Flush an incomplete transaction with enough time left for confirmation.
///
/// `send_transaction_batch` requires at least four slots.  The old two-slot
/// margin could therefore never send a partial batch: proof results would sit
/// in `pending_batch` until eligibility ended and then be handed back to the
/// cache.  Ten slots leaves roughly four seconds on the default slot schedule
/// and gives the confirmation loop useful headroom.
const FLUSH_MARGIN_SLOTS: u64 = 10;
/// Late proofs are an optimization. Do not let a proof job that never releases
/// its sender keep the tree cache in the warming state indefinitely.
pub(super) const LATE_PROOF_COLLECTION_TIMEOUT: Duration = Duration::from_secs(30);
/// Keep receiving proof results after releasing the warming lock. This matches
/// the default maximum prover wait and bounds detached collector lifetime when
/// a producer retains its sender indefinitely.
pub(super) const LATE_PROOF_RETENTION_TIMEOUT: Duration = Duration::from_secs(600);

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
        common::send_transaction_batch,
        proof_cache::{ProofCacheWarmup, ProofCollection, SharedProofCache},
        proof_worker::ProofJobResult,
        BatchContext,
    },
    transaction_timing::scheduled_confirmation_deadline,
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

type TxSenderTaskResult = std::result::Result<TxSenderResult, ForesterError>;
/// (instruction, seq, old_root, new_root)
type PendingProof = (BatchInstruction, u64, [u8; 32], [u8; 32]);
pub(crate) type TxSenderTask = JoinHandle<TxSenderTaskResult>;

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

/// Entry in the ordered proof buffer: instruction + timing info + roots for caching
#[derive(Clone)]
struct BufferEntry {
    instruction: BatchInstruction,
    old_root: [u8; 32],
    new_root: [u8; 32],
    submitted_at: std::time::Instant,
}

struct OrderedProofBuffer {
    buffer: Vec<Option<BufferEntry>>,
    base_seq: u64,
    len: usize,
    head: usize,
}

impl OrderedProofBuffer {
    fn new(capacity: usize, base_seq: u64) -> Self {
        Self {
            buffer: (0..capacity).map(|_| None).collect(),
            base_seq,
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

    #[allow(clippy::too_many_arguments)]
    fn insert(
        &mut self,
        seq: u64,
        instruction: BatchInstruction,
        old_root: [u8; 32],
        new_root: [u8; 32],
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
            old_root,
            new_root,
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

    fn drain_all(&mut self) -> Vec<(u64, BufferEntry)> {
        let mut entries = Vec::with_capacity(self.len);
        for offset in 0..self.buffer.len() {
            let index = (self.head + offset) % self.buffer.len();
            if let Some(entry) = self.buffer[index].take() {
                entries.push((self.base_seq + offset as u64, entry));
            }
        }
        self.len = 0;
        entries
    }
}

pub struct TxSender<R: Rpc> {
    context: BatchContext<R>,
    buffer: OrderedProofBuffer,
    zkp_batch_size: u64,
    last_seen_root: [u8; 32],
    /// Old root of the next proof expected in sequence.
    next_expected_root: [u8; 32],
    pending_batch: Vec<PendingProof>,
    /// Earliest submission time in the pending batch (for end-to-end latency)
    pending_batch_earliest_submit: Option<std::time::Instant>,
    proof_timings: ProofTimings,
    /// Optional cache to save unused proofs when epoch ends (for reuse in next epoch)
    proof_cache: Option<Arc<SharedProofCache>>,
    /// Maximum instructions per transaction
    ixs_per_tx: usize,
}

impl<R: Rpc> TxSender<R> {
    pub(crate) fn spawn(
        context: BatchContext<R>,
        proof_rx: mpsc::Receiver<ProofJobResult>,
        zkp_batch_size: u64,
        last_seen_root: [u8; 32],
        proof_cache: Option<Arc<SharedProofCache>>,
        initial_seq: u64,
    ) -> TxSenderTask {
        let ixs_per_tx = if context.address_lookup_tables.is_empty() {
            V2_IXS_PER_TX_WITHOUT_LUT
        } else {
            V2_IXS_PER_TX_WITH_LUT
        };

        let sender = Self {
            context,
            buffer: OrderedProofBuffer::new(MAX_BUFFER_SIZE, initial_seq),
            zkp_batch_size,
            last_seen_root,
            next_expected_root: last_seen_root,
            pending_batch: Vec::with_capacity(ixs_per_tx),
            pending_batch_earliest_submit: None,
            proof_timings: ProofTimings::default(),
            proof_cache,
            ixs_per_tx,
        };

        tokio::spawn(async move { sender.run(proof_rx).await })
    }

    #[inline]
    fn should_flush_due_to_time_at(&self, current_slot: u64) -> bool {
        let slots_remaining = self
            .context
            .eligibility_end_slot()
            .saturating_sub(current_slot);
        slots_remaining <= FLUSH_MARGIN_SLOTS
    }

    /// `send_transaction_batch` rejects batches without eligibility and the
    /// minimum confirmation budget, so proofs must be handed off before then.
    #[inline]
    fn has_send_budget_at(&self, current_slot: u64) -> bool {
        let slots_remaining = self
            .context
            .eligibility_end_slot()
            .saturating_sub(current_slot);
        scheduled_confirmation_deadline(slots_remaining).is_some()
    }

    async fn run(mut self, mut proof_rx: mpsc::Receiver<ProofJobResult>) -> TxSenderTaskResult {
        let (batch_tx, mut batch_rx) =
            mpsc::unbounded_channel::<(Vec<PendingProof>, Option<std::time::Instant>)>();

        let sender_context = self.context.clone();
        let mut sender_last_root = self.last_seen_root;
        let zkp_batch_size_val = self.zkp_batch_size;
        let sender_cache = self.proof_cache.clone();

        let sender_handle = tokio::spawn(async move {
            let mut sender_processed = 0usize;
            let mut total_tx_sending_duration = Duration::ZERO;
            while let Some((batch, batch_earliest_submit)) = batch_rx.recv().await {
                let items_count: usize = batch
                    .iter()
                    .map(|(instr, _, _, _)| instr.items_count())
                    .sum();
                let first_seq = batch.first().map(|(_, s, _, _)| *s).unwrap_or(0);
                let last_seq = batch.last().map(|(_, s, _, _)| *s).unwrap_or(0);

                let mut all_instructions: Vec<Instruction> = Vec::new();
                let mut last_root: Option<[u8; 32]> = None;
                let mut append_count = 0usize;
                let mut nullify_count = 0usize;

                for (instr, _seq, _, _) in &batch {
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
                        let items_processed = items_count * zkp_batch_size_val as usize;
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
                        if e.is_not_in_active_phase() {
                            warn!("Active phase ended while sending tx, stopping sender loop");
                            let mut unsent = batch;
                            while let Ok((queued, _)) = batch_rx.try_recv() {
                                unsent.extend(queued);
                            }
                            let returned =
                                return_unsent_to_cache(sender_cache.as_ref(), unsent).await;
                            return Ok((sender_processed, total_tx_sending_duration, returned));
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
            Ok((sender_processed, total_tx_sending_duration, 0))
        });

        loop {
            if sender_handle.is_finished() {
                break;
            }

            let current_slot = self.context.slot_tracker.estimated_current_slot();
            if !self.has_send_budget_at(current_slot) {
                let proofs_saved = self.handoff_proofs_to_cache(proof_rx, None).await;
                info!(
                    "Active phase ended for epoch {}, stopping tx sender before recv (saved {} proofs to cache)",
                    self.context.epoch, proofs_saved
                );
                drop(batch_tx);
                let (items_processed, tx_sending_duration, returned) =
                    sender_handle.await.map_err(ForesterError::from)??;
                return Ok(TxSenderResult {
                    items_processed,
                    proof_timings: self.proof_timings,
                    proofs_saved_to_cache: proofs_saved + returned,
                    tx_sending_duration,
                });
            }

            let result = match tokio::time::timeout(Duration::from_secs(1), proof_rx.recv()).await {
                Ok(Some(r)) => r,
                Ok(None) => break,
                Err(_) => {
                    // A partial batch may have been waiting since the previous
                    // proof result. Re-check the slot on every receive timeout;
                    // otherwise it is only flushed when another proof arrives,
                    // which may be after the eligibility window has ended.
                    let current_slot = self.context.slot_tracker.estimated_current_slot();
                    // The slot may have jumped past the send budget during the
                    // wait. The check at the top of the loop hands the batch off.
                    if !self.pending_batch.is_empty()
                        && self.has_send_budget_at(current_slot)
                        && self.should_flush_due_to_time_at(current_slot)
                    {
                        let batch = std::mem::replace(
                            &mut self.pending_batch,
                            Vec::with_capacity(self.ixs_per_tx),
                        );
                        let earliest = self.pending_batch_earliest_submit.take();

                        if let Err(mpsc::error::SendError((batch, earliest))) =
                            batch_tx.send((batch, earliest))
                        {
                            self.pending_batch = batch;
                            self.pending_batch_earliest_submit = earliest;
                            break;
                        }
                    }
                    continue;
                }
            };

            let current_slot = self.context.slot_tracker.estimated_current_slot();

            if !self.has_send_budget_at(current_slot) {
                let proofs_saved = self.handoff_proofs_to_cache(proof_rx, Some(result)).await;
                info!(
                    "Active phase ended for epoch {}, stopping tx sender (saved {} proofs to cache)",
                    self.context.epoch, proofs_saved
                );
                drop(batch_tx);
                let (items_processed, tx_sending_duration, returned) =
                    sender_handle.await.map_err(ForesterError::from)??;
                return Ok(TxSenderResult {
                    items_processed,
                    proof_timings: self.proof_timings,
                    proofs_saved_to_cache: proofs_saved + returned,
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
                    return Err(anyhow::anyhow!("Proof failed seq={}: {}", result.seq, e).into());
                }
            };

            if self.buffer.len() >= self.buffer.capacity() {
                return Err(anyhow::anyhow!("Proof buffer overflow").into());
            }
            if !self.buffer.insert(
                result.seq,
                instruction,
                result.old_root,
                result.new_root,
                result.submitted_at,
            ) {
                warn!("Failed to insert proof seq={}", result.seq);
            }

            while let Some(entry) = self.buffer.pop_next() {
                let seq = self.buffer.expected_seq() - 1;
                self.next_expected_root = entry.new_root;
                self.pending_batch
                    .push((entry.instruction, seq, entry.old_root, entry.new_root));
                self.pending_batch_earliest_submit =
                    Some(match self.pending_batch_earliest_submit {
                        None => entry.submitted_at,
                        Some(existing) => existing.min(entry.submitted_at),
                    });

                let should_send = self.pending_batch.len() >= self.ixs_per_tx
                    || (!self.pending_batch.is_empty()
                        && self.should_flush_due_to_time_at(current_slot));

                if should_send {
                    let batch = std::mem::replace(
                        &mut self.pending_batch,
                        Vec::with_capacity(self.ixs_per_tx),
                    );
                    let earliest = self.pending_batch_earliest_submit.take();

                    if let Err(mpsc::error::SendError((batch, earliest))) =
                        batch_tx.send((batch, earliest))
                    {
                        self.pending_batch = batch;
                        self.pending_batch_earliest_submit = earliest;
                        break;
                    }
                }
            }
        }

        if sender_handle.is_finished() {
            // The sender only stops while batches can still be queued when the
            // send budget ended (or on error). Keep the proofs it never received.
            drop(batch_tx);
            let (items_processed, tx_sending_duration, returned) =
                sender_handle.await.map_err(ForesterError::from)??;
            let proofs_saved = self.handoff_proofs_to_cache(proof_rx, None).await;
            return Ok(TxSenderResult {
                items_processed,
                proof_timings: self.proof_timings,
                proofs_saved_to_cache: proofs_saved + returned,
                tx_sending_duration,
            });
        }

        if !self.pending_batch.is_empty() {
            let batch =
                std::mem::replace(&mut self.pending_batch, Vec::with_capacity(self.ixs_per_tx));
            let earliest = self.pending_batch_earliest_submit.take();
            let _ = batch_tx.send((batch, earliest));
        }

        drop(batch_tx);
        let (items_processed, tx_sending_duration, returned) =
            sender_handle.await.map_err(ForesterError::from)??;

        Ok(TxSenderResult {
            items_processed,
            proof_timings: self.proof_timings,
            proofs_saved_to_cache: returned,
            tx_sending_duration,
        })
    }

    /// Moves proof result ownership to the per-tree cache when eligibility ends.
    ///
    /// Results that are already available are cached synchronously. The receiver is
    /// then kept alive by a background task so proofs that finish after this sender
    /// returns are not lost. The cache remains in the warming state until every
    /// submitted proof job has dropped its sender.
    async fn handoff_proofs_to_cache(
        &mut self,
        mut proof_rx: mpsc::Receiver<ProofJobResult>,
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

        let warmup = cache.start_warming(self.next_expected_root).await;
        let collection = cache.start_collecting(self.next_expected_root);

        // Save proofs from pending_batch (already processed but not yet sent)
        for (instruction, seq, old_root, new_root) in self.pending_batch.drain(..) {
            collection.record_proof(old_root, new_root);
            warmup.add_proof(seq, old_root, new_root, instruction).await;
            saved += 1;
        }

        // Save proofs from the reorder buffer (received but waiting for in-order processing)
        for (seq, entry) in self.buffer.drain_all() {
            collection.record_proof(entry.old_root, entry.new_root);
            warmup
                .add_proof(seq, entry.old_root, entry.new_root, entry.instruction)
                .await;
            saved += 1;
        }

        // Save the current result if provided, then drain remaining proofs from the channel
        let mut available: Vec<_> = current_result.into_iter().collect();
        while let Ok(result) = proof_rx.try_recv() {
            available.push(result);
        }
        for result in available {
            match result.result {
                Ok(instruction) => {
                    collection.record_proof(result.old_root, result.new_root);
                    warmup
                        .add_proof(result.seq, result.old_root, result.new_root, instruction)
                        .await;
                    saved += 1;
                }
                Err(_) => collection.record_failure(result.old_root),
            }
        }

        if saved > 0 {
            info!(
                "Saved {} available proofs to cache and waiting for late results (root: {:?})",
                saved,
                &self.next_expected_root[..4]
            );
        } else {
            info!(
                "Waiting for late proof results to warm cache (root: {:?})",
                &self.next_expected_root[..4]
            );
        }

        // Dropping the JoinHandle detaches the collector so it can finish warming the cache.
        drop(spawn_late_proof_collector(
            warmup,
            collection,
            cache.clone(),
            proof_rx,
            self.context.merkle_tree,
            LATE_PROOF_COLLECTION_TIMEOUT,
            LATE_PROOF_RETENTION_TIMEOUT,
        ));

        saved
    }
}

/// Keep proofs from batches rejected because the send budget ended.
async fn return_unsent_to_cache(
    cache: Option<&Arc<SharedProofCache>>,
    unsent: Vec<PendingProof>,
) -> usize {
    let Some(cache) = cache else {
        debug!(
            "No proof cache available, discarding {} unsent proofs",
            unsent.len()
        );
        return 0;
    };
    let returned = unsent.len();
    for (instruction, seq, old_root, new_root) in unsent {
        cache
            .add_late_proof(seq, old_root, new_root, instruction)
            .await;
    }
    returned
}

fn spawn_late_proof_collector(
    warmup: ProofCacheWarmup,
    collection: ProofCollection,
    cache: Arc<SharedProofCache>,
    mut proof_rx: mpsc::Receiver<ProofJobResult>,
    tree: solana_sdk::pubkey::Pubkey,
    collection_timeout: Duration,
    retention_timeout: Duration,
) -> JoinHandle<usize> {
    tokio::spawn(async move {
        let mut saved = 0usize;
        let warming = async {
            while let Some(result) = proof_rx.recv().await {
                match result.result {
                    Ok(instruction) => {
                        collection.record_proof(result.old_root, result.new_root);
                        warmup
                            .add_proof(result.seq, result.old_root, result.new_root, instruction)
                            .await;
                        saved += 1;
                    }
                    Err(error) => {
                        collection.record_failure(result.old_root);
                        warn!(
                            tree = %tree,
                            seq = result.seq,
                            error = %error,
                            "Late proof failed while warming cache"
                        );
                    }
                }
            }
        };

        let collection_timed_out = tokio::time::timeout(collection_timeout, warming)
            .await
            .is_err();

        // Release the tree immediately at the scheduling deadline. The
        // receiver remains alive below so completed work is still retained.
        warmup.finish().await;

        if collection_timed_out {
            warn!(
                tree = %tree,
                timeout_ms = collection_timeout.as_millis(),
                late_proofs_cached = saved,
                "Late proof collection timed out; released cache warming state and retaining later results"
            );

            let mut retained = 0usize;
            let retention = async {
                while let Some(result) = proof_rx.recv().await {
                    match result.result {
                        Ok(instruction) => {
                            collection.record_proof(result.old_root, result.new_root);
                            cache
                                .add_late_proof(
                                    result.seq,
                                    result.old_root,
                                    result.new_root,
                                    instruction,
                                )
                                .await;
                            saved += 1;
                            retained += 1;
                        }
                        Err(error) => {
                            collection.record_failure(result.old_root);
                            warn!(
                                tree = %tree,
                                seq = result.seq,
                                error = %error,
                                "Proof failed after cache warm-up deadline"
                            );
                        }
                    }
                }
            };

            if tokio::time::timeout(retention_timeout, retention)
                .await
                .is_err()
            {
                warn!(
                    tree = %tree,
                    timeout_ms = retention_timeout.as_millis(),
                    retained_late_proofs = retained,
                    "Late proof retention window expired"
                );
            } else if retained > 0 {
                info!(
                    tree = %tree,
                    retained_late_proofs = retained,
                    "Retained proofs that completed after cache warm-up deadline"
                );
            }
        }

        let total_cached_proofs = cache.len().await;
        info!(
            tree = %tree,
            late_proofs_cached = saved,
            total_cached_proofs,
            "Late proof collection completed"
        );
        saved
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;

    use forester_utils::{
        forester_epoch::{EpochPhases, Phase},
        rpc_pool::SolanaRpcPoolBuilder,
    };
    use light_client::rpc::LightClient;
    use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair};

    use super::*;
    use crate::{
        processor::{tx_cache::ProcessedHashCache, v2::ProverConfig},
        slot_tracker::SlotTracker,
        smart_transaction::TransactionPolicy,
    };

    #[test]
    fn handoff_drains_out_of_order_proofs_across_sequence_gaps() {
        let mut buffer = OrderedProofBuffer::new(4, 10);
        let now = std::time::Instant::now();
        assert!(buffer.insert(
            10,
            BatchInstruction::Append(Vec::new()),
            [1; 32],
            [2; 32],
            now
        ));
        assert!(buffer.pop_next().is_some());
        // Includes a wrapped ring-buffer entry, but seq=11 hasn't arrived.
        assert!(buffer.insert(
            12,
            BatchInstruction::Append(Vec::new()),
            [3; 32],
            [4; 32],
            now
        ));
        assert!(buffer.insert(
            14,
            BatchInstruction::Append(Vec::new()),
            [5; 32],
            [6; 32],
            now
        ));
        assert!(buffer.pop_next().is_none());
        let entries = buffer.drain_all();
        assert_eq!(
            entries.iter().map(|(seq, _)| *seq).collect::<Vec<_>>(),
            vec![12, 14]
        );
        assert_eq!(buffer.len(), 0);
    }

    fn proof_result(seq: u64, old_root: [u8; 32], new_root: [u8; 32]) -> ProofJobResult {
        ProofJobResult {
            seq,
            result: Ok(BatchInstruction::Append(Vec::new())),
            old_root,
            new_root,
            proof_duration_ms: 1,
            round_trip_ms: 1,
            submitted_at: std::time::Instant::now(),
        }
    }

    #[tokio::test]
    async fn late_proof_result_finishes_warming_cache() {
        let tree = solana_sdk::pubkey::Pubkey::new_unique();
        let base_root = [1u8; 32];
        let next_root = [2u8; 32];
        let cache = Arc::new(SharedProofCache::new(tree));
        let warmup = cache.start_warming(base_root).await;

        let (proof_tx, proof_rx) = mpsc::channel(1);
        let collector = spawn_late_proof_collector(
            warmup,
            cache.start_collecting(base_root),
            cache.clone(),
            proof_rx,
            tree,
            LATE_PROOF_COLLECTION_TIMEOUT,
            LATE_PROOF_RETENTION_TIMEOUT,
        );

        assert!(cache.is_warming().await);
        proof_tx
            .send(proof_result(0, base_root, next_root))
            .await
            .unwrap();
        drop(proof_tx);

        assert_eq!(collector.await.unwrap(), 1);
        assert!(!cache.is_warming().await);
        let cached = cache.ready_chain(&base_root, &[]).await.unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].old_root, base_root);
        assert_eq!(cached[0].new_root, next_root);
    }

    #[tokio::test]
    async fn failed_late_proof_does_not_block_cache_completion() {
        let tree = solana_sdk::pubkey::Pubkey::new_unique();
        let base_root = [3u8; 32];
        let cache = Arc::new(SharedProofCache::new(tree));
        let warmup = cache.start_warming(base_root).await;

        let (proof_tx, proof_rx) = mpsc::channel(1);
        let collector = spawn_late_proof_collector(
            warmup,
            cache.start_collecting(base_root),
            cache.clone(),
            proof_rx,
            tree,
            LATE_PROOF_COLLECTION_TIMEOUT,
            LATE_PROOF_RETENTION_TIMEOUT,
        );
        proof_tx
            .send(ProofJobResult {
                seq: 0,
                result: Err("proof failed".to_string()),
                old_root: base_root,
                new_root: [4u8; 32],
                proof_duration_ms: 1,
                round_trip_ms: 1,
                submitted_at: std::time::Instant::now(),
            })
            .await
            .unwrap();
        drop(proof_tx);

        assert_eq!(collector.await.unwrap(), 0);
        assert!(!cache.is_warming().await);
        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn retained_proof_sender_cannot_block_cache_completion() {
        let tree = solana_sdk::pubkey::Pubkey::new_unique();
        let base_root = [5u8; 32];
        let cache = Arc::new(SharedProofCache::new(tree));
        let warmup = cache.start_warming(base_root).await;

        let (proof_tx, proof_rx) = mpsc::channel(1);
        let collector = spawn_late_proof_collector(
            warmup,
            cache.start_collecting(base_root),
            cache.clone(),
            proof_rx,
            tree,
            Duration::from_millis(20),
            Duration::from_secs(1),
        );

        assert!(cache.is_warming().await);
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert!(!cache.is_warming().await);
        assert!(!proof_tx.is_closed());
        assert!(cache.has_pending_proofs().await);

        let next_root = [6u8; 32];
        proof_tx
            .send(proof_result(0, base_root, next_root))
            .await
            .unwrap();
        drop(proof_tx);

        assert_eq!(collector.await.unwrap(), 1);
        let cached = cache.ready_chain(&base_root, &[]).await.unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].new_root, next_root);
        assert!(!cache.has_pending_proofs().await);
    }

    const ELIGIBILITY_END_SLOT: u64 = 1_000;

    async fn test_sender_context(
        current_slot: u64,
    ) -> (BatchContext<LightClient>, Arc<SlotTracker>) {
        let rpc_pool = SolanaRpcPoolBuilder::<LightClient>::new()
            .url("http://127.0.0.1:1".to_string())
            .commitment(CommitmentConfig::confirmed())
            .build()
            .await
            .unwrap();
        let slot_tracker = Arc::new(SlotTracker::new(current_slot, Duration::from_secs(60)));
        let prover_url = "http://127.0.0.1:1".to_string();
        let active = Phase {
            start: 0,
            end: 1_000_000,
        };
        let context = BatchContext {
            rpc_pool: Arc::new(rpc_pool),
            authority: Arc::new(Keypair::new()),
            run_id: Arc::from("test"),
            derivation: Pubkey::new_unique(),
            epoch: 0,
            merkle_tree: Pubkey::new_unique(),
            output_queue: Pubkey::new_unique(),
            prover_config: Arc::new(ProverConfig {
                append_url: prover_url.clone(),
                update_url: prover_url.clone(),
                address_append_url: prover_url,
                api_key: None,
                polling_interval: Duration::from_secs(1),
                max_wait_time: Duration::from_secs(1),
                max_concurrent_jobs: 1,
                network: "test".to_string(),
            }),
            ops_cache: Arc::new(tokio::sync::Mutex::new(ProcessedHashCache::new(60))),
            epoch_phases: EpochPhases {
                registration: active.clone(),
                active: active.clone(),
                report_work: active.clone(),
                post: active,
            },
            slot_tracker: slot_tracker.clone(),
            input_queue_hint: None,
            output_queue_hint: None,
            num_proof_workers: 1,
            forester_eligibility_end_slot: Arc::new(AtomicU64::new(ELIGIBILITY_END_SLOT)),
            address_lookup_tables: Arc::new(Vec::new()),
            transaction_policy: TransactionPolicy::default(),
            max_batches_per_tree: 1,
        };
        (context, slot_tracker)
    }

    #[tokio::test]
    async fn partial_batch_is_cached_when_slot_jumps_during_receive_wait() {
        let (context, slot_tracker) = test_sender_context(ELIGIBILITY_END_SLOT - 500).await;
        let cache = Arc::new(SharedProofCache::new(context.merkle_tree));
        let root = [1u8; 32];
        let (proof_tx, proof_rx) = mpsc::channel(4);
        let sender = TxSender::spawn(context, proof_rx, 10, root, Some(cache.clone()), 0);

        // A single proof stays in the partial batch: far from the flush margin.
        proof_tx
            .send(proof_result(0, root, [2u8; 32]))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        // The slot jumps past eligibility while the sender waits for the next proof.
        slot_tracker.update(ELIGIBILITY_END_SLOT);

        let result = tokio::time::timeout(Duration::from_secs(5), sender)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(result.items_processed, 0);
        assert_eq!(result.proofs_saved_to_cache, 1);
        let cached = cache.ready_chain(&root, &[]).await.unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].new_root, [2u8; 32]);
        drop(proof_tx);
    }

    #[tokio::test]
    async fn proofs_are_cached_once_confirmation_budget_is_exhausted() {
        // Still eligible, but fewer slots remain than a send needs to confirm.
        let (context, _slot_tracker) = test_sender_context(ELIGIBILITY_END_SLOT - 2).await;
        let cache = Arc::new(SharedProofCache::new(context.merkle_tree));
        let root = [3u8; 32];
        let (proof_tx, proof_rx) = mpsc::channel(4);
        proof_tx
            .send(proof_result(0, root, [4u8; 32]))
            .await
            .unwrap();
        let sender = TxSender::spawn(context, proof_rx, 10, root, Some(cache.clone()), 0);

        let result = tokio::time::timeout(Duration::from_secs(5), sender)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(result.items_processed, 0);
        assert_eq!(result.proofs_saved_to_cache, 1);
        assert_eq!(cache.ready_chain(&root, &[]).await.unwrap().len(), 1);
        // The next proof is still pending, so the collection holds the tree.
        assert!(cache.awaits_proof_for(&[4u8; 32]));
        drop(proof_tx);
    }
}
