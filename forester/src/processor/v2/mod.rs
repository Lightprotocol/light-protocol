mod batch_job_builder;
pub mod common;
pub mod errors;
mod helpers;
mod processor;
pub mod proof_cache;
mod proof_worker;
mod root_guard;
pub mod strategy;
mod tx_sender;

pub use common::{BatchContext, ProverConfig};
pub use processor::QueueProcessor;
pub use proof_cache::{CachedProof, SharedProofCache};
pub use tx_sender::{BatchInstruction, ProofTimings, TxSenderResult};

use crate::epoch_manager::ProcessingMetrics;

#[derive(Debug, Clone, Default)]
pub struct ProcessingResult {
    pub items_processed: usize,
    pub metrics: ProcessingMetrics,
}
