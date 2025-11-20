/// Error types for the state tree coordinator.
use thiserror::Error;

/// Errors that can occur during coordinator operations.
#[derive(Debug, Error)]
pub enum CoordinatorError {
    /// Root changed during processing, indicating a concurrent forester update.
    #[error("Root changed during {phase}: expected {expected:?}, got {actual:?}")]
    RootChanged {
        phase: String,
        expected: [u8; 8],
        actual: [u8; 8],
    },

    /// Photon indexer data is stale compared to on-chain state.
    #[error("Photon staleness detected: {queue_type} queue initial_root {photon_root:?} != on-chain root {onchain_root:?}")]
    PhotonStale {
        queue_type: String,
        photon_root: [u8; 8],
        onchain_root: [u8; 8],
    },

    /// Photon indexer data doesn't start at the expected batch index.
    #[error("Photon {queue_type} queue data mismatch: expected start_index={expected_start}, got first_queue_index={actual_start}.")]
    PhotonIndexMismatch {
        queue_type: String,
        expected_start: u64,
        actual_start: u64,
    },

    /// Hash chain validation failed for nullify batch.
    #[error(
        "Hash chain mismatch in batch {batch_index}: expected {expected:?}, computed {computed:?}"
    )]
    HashChainMismatch {
        batch_index: usize,
        expected: [u8; 8],
        computed: [u8; 8],
    },

    /// Proof generation failed.
    #[error("Proof generation failed for {batch_type} batch {index}: {source}")]
    ProofGenerationFailed {
        batch_type: String,
        index: usize,
        source: anyhow::Error,
    },

    /// Constraint error during proof generation, likely due to stale tree state.
    #[error("Constraint error in batch {batch_index}: {details}")]
    ConstraintError { batch_index: usize, details: String },

    /// Transaction submission failed.
    #[error("Transaction submission failed: {0}")]
    TransactionFailed(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, CoordinatorError>;

impl CoordinatorError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            CoordinatorError::RootChanged { .. }
                | CoordinatorError::PhotonStale { .. }
                | CoordinatorError::PhotonIndexMismatch { .. }
                | CoordinatorError::HashChainMismatch { .. }
                | CoordinatorError::ConstraintError { .. }
        )
    }

    pub fn requires_resync(&self) -> bool {
        matches!(
            self,
            CoordinatorError::RootChanged { .. }
                | CoordinatorError::PhotonIndexMismatch { .. }
                | CoordinatorError::HashChainMismatch { .. }        )
    }
}
