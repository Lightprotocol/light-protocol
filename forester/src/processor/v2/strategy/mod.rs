use async_trait::async_trait;
use light_client::rpc::Rpc;

use crate::processor::v2::{proof_worker::ProofInput, BatchContext, QueueWork};

mod address;
mod state;

pub use address::AddressTreeStrategy;
pub use state::StateTreeStrategy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitType {
    Append,
    Nullify,
    AddressAppend,
}

#[derive(Debug)]
pub struct QueueData<T> {
    pub staging_tree: T,
    pub initial_root: [u8; 32],
    pub num_batches: usize,
}

#[async_trait]
pub trait TreeStrategy<R: Rpc>: Send + Sync + Clone + std::fmt::Debug + 'static {
    type StagingTree: Send + 'static;

    fn name(&self) -> &'static str;
    fn circuit_type(&self, queue_data: &Self::StagingTree) -> CircuitType;

    /// Get the circuit type for a specific batch index.
    /// This is needed for combined APPEND+NULLIFY processing where the circuit type
    /// changes mid-batch based on batch_idx.
    fn circuit_type_for_batch(&self, queue_data: &Self::StagingTree, batch_idx: usize) -> CircuitType {
        // Default implementation just delegates to circuit_type (batch_idx used by StateTreeStrategy override)
        let _ = batch_idx;
        self.circuit_type(queue_data)
    }

    async fn fetch_zkp_batch_size(&self, context: &BatchContext<R>) -> crate::Result<u64>;

    /// Fetch the current on-chain root for this tree type.
    /// Used to initialize processor state and validate indexer data.
    async fn fetch_onchain_root(&self, context: &BatchContext<R>) -> crate::Result<[u8; 32]>;

    async fn fetch_queue_data(
        &self,
        context: &BatchContext<R>,
        queue_work: &QueueWork,
        max_batches: usize,
        zkp_batch_size: u64,
    ) -> crate::Result<Option<QueueData<Self::StagingTree>>>;

    fn build_proof_job(
        &self,
        queue_data: &mut Self::StagingTree,
        batch_idx: usize,
        start: usize,
        zkp_batch_size: u64,
        epoch: u64,
        tree: &str,
    ) -> crate::Result<(ProofInput, [u8; 32])>;
}
