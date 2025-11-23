pub mod common;
mod helpers;
mod processor;
pub mod proof_cache;
mod proof_worker;
pub mod strategy;
mod tx_sender;

use crate::{epoch_manager::ProcessingMetrics};

pub use common::{BatchContext, ProverConfig, QueueWork};
pub use proof_cache::{CachedProof, SharedProofCache};

pub use processor::{is_constraint_error, is_hashchain_mismatch, QueueProcessor};
pub use tx_sender::{BatchInstruction, ProofTimings, TxSenderResult};

#[derive(Debug, Clone, Default)]
pub struct ProcessingResult {
    pub items_processed: usize,
    pub metrics: ProcessingMetrics,
}
