use solana_sdk::pubkey::Pubkey;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct IterationTelemetry {
    pub tree: Pubkey,
    pub prepare_duration: Duration,
    pub prove_duration: Duration,
    pub submit_duration: Duration,
    pub total_duration: Duration,
    pub append_batches: usize,
    pub nullify_batches: usize,
    pub items_processed: usize,
}

impl IterationTelemetry {
    pub fn total_batches(&self) -> usize {
        self.append_batches + self.nullify_batches
    }

    /// Report telemetry to Prometheus metrics
    pub fn report(&self) {
        crate::metrics::observe_iteration_duration(&self.tree, self.total_duration);

        if self.append_batches > 0 {
            crate::metrics::increment_batches_processed(&self.tree, "append", self.append_batches);
        }
        if self.nullify_batches > 0 {
            crate::metrics::increment_batches_processed(
                &self.tree,
                "nullify",
                self.nullify_batches,
            );
        }
    }
}

pub struct QueueTelemetry {
    pub tree: Pubkey,
    pub pending_items: usize,
}

impl QueueTelemetry {
    pub fn report(&self) {
        crate::metrics::update_pending_queue_items(&self.tree, self.pending_items);
    }
}

pub enum CacheEvent {
    StagingAdded,
    StagingEvicted,
    StagingProcessed,
    SpeculativeHit,
    SpeculativeMiss,
    SpeculativeEvicted,
}

impl CacheEvent {
    pub fn report(&self, tree: &Pubkey, reason: &str) {
        match self {
            CacheEvent::StagingAdded => {
                crate::metrics::record_staging_cache_event(tree, "added", reason)
            }
            CacheEvent::StagingEvicted => {
                crate::metrics::record_staging_cache_event(tree, "evicted", reason)
            }
            CacheEvent::StagingProcessed => {
                crate::metrics::record_staging_cache_event(tree, "processed", reason)
            }
            CacheEvent::SpeculativeHit => {
                crate::metrics::record_speculative_event(tree, "hit", reason)
            }
            CacheEvent::SpeculativeMiss => {
                crate::metrics::record_speculative_event(tree, "miss", reason)
            }
            CacheEvent::SpeculativeEvicted => {
                crate::metrics::record_speculative_event(tree, "evicted", reason)
            }
        }
    }
}
