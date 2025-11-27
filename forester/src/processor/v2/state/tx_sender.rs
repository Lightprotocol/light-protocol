use std::collections::BTreeMap;

use borsh::BorshSerialize;
use light_batched_merkle_tree::merkle_tree::{
    InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs,
};
use light_client::rpc::Rpc;
use light_registry::account_compression_cpi::sdk::{
    create_batch_append_instruction, create_batch_nullify_instruction,
    create_batch_update_address_tree_instruction,
};
use solana_sdk::signature::Signer;
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::{info, warn};

use crate::{
    errors::ForesterError,
    processor::v2::{
        common::send_transaction_batch, state::proof_worker::ProofResult, BatchContext,
    },
};

#[derive(Debug)]
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
        };

        tokio::spawn(async move { sender.run(proof_rx).await })
    }

    async fn run(mut self, mut proof_rx: mpsc::Receiver<ProofResult>) -> crate::Result<usize> {
        let mut processed = 0usize;

        while let Some(result) = proof_rx.recv().await {
            self.buffer.insert(result.seq, result.instruction);

            while let Some(instr) = self.buffer.remove(&self.expected_seq) {
                let (instructions, expected_root) = match &instr {
                    BatchInstruction::Append(proofs) => {
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

                let instr_type = match &instr {
                    BatchInstruction::Append(_) => "Append",
                    BatchInstruction::Nullify(_) => "Nullify",
                    BatchInstruction::AddressAppend(_) => "AddressAppend",
                };

                match send_transaction_batch(&self.context, instructions).await {
                    Ok(sig) => {
                        if let Some(root) = expected_root {
                            self.last_seen_root = root;
                        }
                        processed += self.zkp_batch_size as usize;
                        self.expected_seq += 1;
                        info!(
                            "tx sent: {} type={} root={:?} seq={} epoch={}",
                            sig,
                            instr_type,
                            self.last_seen_root,
                            self.expected_seq,
                            self.context.epoch
                        );
                    }
                    Err(e) => {
                        warn!("tx error {} epoch {}", e, self.context.epoch);
                        return if let Some(ForesterError::NotInActivePhase) =
                            e.downcast_ref::<ForesterError>()
                        {
                            warn!("Active phase ended while sending tx, stopping sender loop");
                            Ok(processed)
                        } else {
                            Err(e)
                        };
                    }
                }
            }
        }

        Ok(processed)
    }
}
