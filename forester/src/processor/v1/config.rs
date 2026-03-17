use light_client::rpc::RetryConfig;

use crate::config::QueueConfig;

#[derive(Debug, Clone, Copy)]
pub struct CapConfig {
    pub rec_fee_microlamports_per_cu: u64,
    pub min_fee_lamports: u64,
    pub max_fee_lamports: u64,
    pub compute_unit_limit: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct SendBatchedTransactionsConfig {
    pub num_batches: u64,
    pub build_transaction_batch_config: BuildTransactionBatchConfig,
    pub queue_config: QueueConfig,
    pub retry_config: RetryConfig,
    pub light_slot_length: u64,
    pub confirmation_poll_interval: std::time::Duration,
    pub confirmation_max_attempts: usize,
}

/// Pending-item threshold below which the forester only emits *paired*
/// state-nullify transactions, dropping unpaired singles.  When the queue is
/// nearly empty there is no urgency, so we save a transaction by waiting for
/// the next cycle when the single can potentially be paired.
pub const PAIRS_ONLY_THRESHOLD: u64 = 4_000;

#[derive(Debug, Clone, Copy)]
pub struct BuildTransactionBatchConfig {
    pub batch_size: u64,
    pub compute_unit_price: Option<u64>,
    pub compute_unit_limit: Option<u32>,
    pub enable_priority_fees: bool,
    pub max_concurrent_sends: Option<usize>,
    /// When `true`, only emit paired state-nullify transactions.
    /// Unpaired singles are dropped and retried in the next cycle.
    /// Computed at runtime: `pairs_only = total_pending < PAIRS_ONLY_THRESHOLD`.
    pub pairs_only: bool,
}
