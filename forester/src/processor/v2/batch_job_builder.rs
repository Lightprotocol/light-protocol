use crate::processor::v2::proof_worker::ProofInput;

pub trait BatchJobBuilder {
    /// Build proof job for a batch. Returns:
    /// - `Ok(Some((input, root)))` - batch processed, proof job created
    /// - `Ok(None)` - batch should be skipped (e.g., overlap with already-processed data)
    /// - `Err(...)` - fatal error, stop processing
    fn build_proof_job(
        &mut self,
        batch_idx: usize,
        start: usize,
        zkp_batch_size: u64,
        epoch: u64,
        tree: &str,
    ) -> crate::Result<Option<(ProofInput, [u8; 32])>>;

    fn available_batches(&self, zkp_batch_size: u64) -> usize {
        let _ = zkp_batch_size;
        usize::MAX
    }
}
