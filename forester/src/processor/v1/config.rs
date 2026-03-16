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

#[derive(Debug, Clone, Copy)]
pub struct BuildTransactionBatchConfig {
    pub batch_size: u64,
    pub compute_unit_price: Option<u64>,
    pub compute_unit_limit: Option<u32>,
    pub enable_priority_fees: bool,
    pub max_concurrent_sends: Option<usize>,
    /// When `true`, only emit paired state-nullify transactions.
    /// Unpaired (single) nullifies are dropped.  The caller sets this based
    /// on queue fullness: `pairs_only = queue_pending < MAX_QUEUE_FULLNESS`.
    pub pairs_only: bool,
}
