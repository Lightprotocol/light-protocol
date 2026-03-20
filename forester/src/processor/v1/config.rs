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
    /// Minimum number of queue items required before processing begins.
    /// Only applies to StateV1 trees. When `None`, processing starts immediately.
    /// When the timeout deadline is reached, returns 0 (re-scheduled next light slot).
    pub min_queue_items: Option<usize>,
    /// When true, fetch leaf indices from the indexer and sort work items by
    /// leaf_index before chunking, so adjacent leaves land in the same batch
    /// for better dedup grouping.
    pub enable_presort: bool,
    /// Number of queue items to process per batch cycle.
    pub work_item_batch_size: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct BuildTransactionBatchConfig {
    pub batch_size: u64,
    pub compute_unit_price: Option<u64>,
    pub compute_unit_limit: Option<u32>,
    pub enable_priority_fees: bool,
    pub max_concurrent_sends: Option<usize>,
}
