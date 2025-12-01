use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

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
use tracing::{info, warn};

use crate::{
    errors::ForesterError,
    processor::v2::{common::send_transaction_batch, proof_worker::ProofResult, BatchContext},
};

#[derive(Debug, Clone)]
pub enum BatchInstruction {
    Append(Vec<InstructionDataBatchAppendInputs>),
    Nullify(Vec<InstructionDataBatchNullifyInputs>),
    AddressAppend(Vec<light_batched_merkle_tree::merkle_tree::InstructionDataAddressAppendInputs>),
}

pub struct TxSender<R: Rpc> {
    context: BatchContext<R>,
    expected_seq: u64,
    buffer: BTreeMap<u64, BatchInstruction>,
    zkp_batch_size: u64,
    last_seen_root: [u8; 32],
    pending_batch: Vec<(BatchInstruction, u64)>, // (instruction, seq)
}

impl<R: Rpc> TxSender<R> {
    pub(crate) fn spawn(
        context: BatchContext<R>,
        proof_rx: mpsc::Receiver<ProofResult>,
        zkp_batch_size: u64,
        last_seen_root: [u8; 32],
    ) -> JoinHandle<crate::Result<usize>> {
        let sender = Self {
            context,
            expected_seq: 0,
            buffer: BTreeMap::new(),
            zkp_batch_size,
            last_seen_root,
            pending_batch: Vec::with_capacity(V2_IXS_PER_TX),
        };

        tokio::spawn(async move { sender.run(proof_rx).await })
    }

    fn should_flush_due_to_time(&self) -> bool {
        let current_slot = self.context.slot_tracker.estimated_current_slot();
        let forester_end = self
            .context
            .forester_eligibility_end_slot
            .load(Ordering::Acquire);
        let eligibility_end_slot = if forester_end > 0 {
            forester_end
        } else {
            self.context.epoch_phases.active.end
        };
        let slots_remaining = eligibility_end_slot.saturating_sub(current_slot);
        slots_remaining < MIN_SLOTS_FOR_BATCHING
    }

    async fn run(mut self, mut proof_rx: mpsc::Receiver<ProofResult>) -> crate::Result<usize> {
        let mut processed = 0usize;

        while let Some(result) = proof_rx.recv().await {
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

            if self.buffer.len() >= MAX_BUFFER_SIZE {
                warn!(
                    "Buffer overflow: {} buffered proofs (max {}). Expected seq={}, oldest buffered={}",
                    self.buffer.len(),
                    MAX_BUFFER_SIZE,
                    self.expected_seq,
                    self.buffer.keys().next().unwrap_or(&0)
                );
                return Err(anyhow::anyhow!(
                    "Proof buffer overflow: possible missing proof for seq={}",
                    self.expected_seq
                ));
            }

            self.buffer.insert(result.seq, instruction);

            while let Some(instr) = self.buffer.remove(&self.expected_seq) {
                let seq = self.expected_seq;
                self.expected_seq += 1;
                self.pending_batch.push((instr, seq));

                // Send batch when:
                // 1. We have enough instructions, OR
                // 2. We're running low on time (epoch ending soon)
                let should_send = self.pending_batch.len() >= V2_IXS_PER_TX
                    || (!self.pending_batch.is_empty() && self.should_flush_due_to_time());

                if should_send {
                    processed += self.send_pending_batch().await?;
                }
            }
        }

        if !self.pending_batch.is_empty() {
            processed += self.send_pending_batch().await?;
        }

        Ok(processed)
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
        let mut instr_type = "";

        for (instr, _seq) in &batch {
            let (instructions, expected_root) = match instr {
                BatchInstruction::Append(proofs) => {
                    instr_type = "Append";
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
                    instr_type = "Nullify";
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
                    instr_type = "AddressAppend";
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
