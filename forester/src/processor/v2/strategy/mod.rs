use async_trait::async_trait;
use light_client::rpc::Rpc;

use crate::processor::v2::{
    batch_job_builder::BatchJobBuilder, proof_worker::ProofInput, BatchContext, QueueWork,
};

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

use light_compressed_account::QueueType;

#[async_trait]
pub trait TreeStrategy<R: Rpc>: Send + Sync + Clone + std::fmt::Debug + 'static {
    type StagingTree: Send + 'static;

    fn name(&self) -> &'static str;
    fn circuit_type(&self, queue_data: &Self::StagingTree) -> CircuitType;

    /// Returns the default queue type for this strategy
    fn queue_type() -> QueueType;

    fn circuit_type_for_batch(
        &self,
        queue_data: &Self::StagingTree,
        batch_idx: usize,
    ) -> CircuitType {
        let _ = batch_idx;
        self.circuit_type(queue_data)
    }

    async fn fetch_zkp_batch_size(&self, context: &BatchContext<R>) -> crate::Result<u64>;

    async fn fetch_onchain_root(&self, context: &BatchContext<R>) -> crate::Result<[u8; 32]>;

    async fn fetch_queue_data(
        &self,
        context: &BatchContext<R>,
        queue_work: &QueueWork,
        max_batches: usize,
        zkp_batch_size: u64,
    ) -> crate::Result<Option<QueueData<Self::StagingTree>>>;

    /// Build proof job for a batch. Returns:
    /// - `Ok(Some((input, root)))` - batch processed, proof job created
    /// - `Ok(None)` - batch should be skipped (e.g., overlap with already-processed data)
    /// - `Err(...)` - fatal error, stop processing
    fn build_proof_job(
        &self,
        queue_data: &mut Self::StagingTree,
        batch_idx: usize,
        zkp_batch_size: u64,
        epoch: u64,
        tree: &str,
    ) -> crate::Result<Option<(ProofInput, [u8; 32])>>
    where
        Self::StagingTree: BatchJobBuilder,
    {
        BatchJobBuilder::build_proof_job(queue_data, batch_idx, zkp_batch_size, epoch, tree)
    }

    /// Returns the number of batches currently available in the staging tree.
    /// For streaming implementations, this may increase as more data is fetched.
    /// Default implementation returns usize::MAX (unlimited).
    fn available_batches(&self, _queue_data: &Self::StagingTree, _zkp_batch_size: u64) -> usize {
        usize::MAX
    }
}
