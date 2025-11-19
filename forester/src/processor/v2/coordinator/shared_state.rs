use std::{collections::HashSet, sync::Arc, time::Duration};

use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcessedBatchId {
    pub batch_index: usize,
    pub zkp_batch_index: u64,
    pub is_append: bool,
    pub start_leaf_index: Option<u64>,
}

impl ProcessedBatchId {
    pub fn append(batch_index: usize, zkp_batch_index: u64) -> Self {
        Self {
            batch_index,
            zkp_batch_index,
            is_append: true,
            start_leaf_index: None,
        }
    }

    pub fn nullify(batch_index: usize, zkp_batch_index: u64) -> Self {
        Self {
            batch_index,
            zkp_batch_index,
            is_append: false,
            start_leaf_index: None,
        }
    }

    pub fn operation_type(&self) -> &'static str {
        if self.is_append {
            "append"
        } else {
            "nullify"
        }
    }
}

pub struct SharedTreeState {
    pub current_root: [u8; 32],
    pub processed_batches: HashSet<ProcessedBatchId>,
    pub metrics: CumulativeMetrics,
}

impl SharedTreeState {
    pub fn new(initial_root: [u8; 32]) -> Self {
        info!(
            "Initializing SharedTreeState with root: {:?}",
            &initial_root[..8]
        );
        Self {
            current_root: initial_root,
            processed_batches: HashSet::new(),
            metrics: CumulativeMetrics::default(),
        }
    }

    pub fn update_root(&mut self, new_root: [u8; 32]) {
        self.current_root = new_root;
    }

    pub fn mark_batch_processed(&mut self, batch_id: ProcessedBatchId) {
        debug!(
            "Marking batch as processed: batch_idx={}, zkp_batch_idx={}, type={}, start_leaf={:?}",
            batch_id.batch_index,
            batch_id.zkp_batch_index,
            batch_id.operation_type(),
            batch_id.start_leaf_index
        );
        self.processed_batches.insert(batch_id);
    }

    pub fn is_batch_processed(&self, batch_id: &ProcessedBatchId) -> bool {
        self.processed_batches.contains(batch_id)
    }

    pub fn reset(
        &mut self,
        on_chain_root: [u8; 32],
        input_queue_batches: &[light_batched_merkle_tree::batch::Batch; 2],
        output_queue_batches: &[light_batched_merkle_tree::batch::Batch; 2],
    ) {
        let processed_count_before = self.processed_batches.len();
        let root_changed = self.current_root != on_chain_root;

        info!(
            "Resetting state: clearing {} processed batches, root_changed={}, syncing to root {:?}",
            processed_count_before,
            root_changed,
            &on_chain_root[..8]
        );

        self.current_root = on_chain_root;

        self.processed_batches.retain(|batch_id| {
            let on_chain_inserted = if batch_id.is_append {
                output_queue_batches
                    .get(batch_id.batch_index)
                    .map(|b| b.get_num_inserted_zkps())
                    .unwrap_or(0)
            } else {
                input_queue_batches
                    .get(batch_id.batch_index)
                    .map(|b| b.get_num_inserted_zkps())
                    .unwrap_or(0)
            };

            let should_keep = batch_id.zkp_batch_index > on_chain_inserted;

            if !should_keep {
                debug!(
                    "Clearing processed batch (confirmed on-chain or failed to insert): batch_idx={}, zkp_batch_idx={}, type={}, on_chain_inserted={}, start_leaf={:?}",
                    batch_id.batch_index,
                    batch_id.zkp_batch_index,
                    batch_id.operation_type(),
                    on_chain_inserted,
                    batch_id.start_leaf_index
                );
            }

            should_keep
        });

        let processed_count_after = self.processed_batches.len();
        let cleared_count = processed_count_before - processed_count_after;

        if cleared_count > 0 {
            info!(
                "Cleared {} processed batches that were confirmed on-chain, {} remaining",
                cleared_count, processed_count_after
            );
        } else if processed_count_after > 0 {
            debug!(
                "Keeping {} processed batches (not yet confirmed on-chain)",
                processed_count_after
            );
        }
    }

    /// Clears all processed batches (used at start of new epoch's active phase).
    pub fn clear_all_processed_batches(&mut self) {
        let count = self.processed_batches.len();
        if count > 0 {
            info!(
                "Clearing all {} processed batches at start of new epoch",
                count
            );
            self.processed_batches.clear();
        }
    }

    pub fn add_iteration_metrics(&mut self, metrics: IterationMetrics) {
        self.metrics.add_iteration(&metrics);
    }

    pub fn get_metrics(&self) -> &CumulativeMetrics {
        &self.metrics
    }

    pub fn merge_metrics(&mut self, other: CumulativeMetrics) {
        self.metrics.iterations += other.iterations;
        self.metrics.total_duration += other.total_duration;
        self.metrics.phase1_total += other.phase1_total;
        self.metrics.phase2_total += other.phase2_total;
        self.metrics.phase3_total += other.phase3_total;
        self.metrics.total_append_batches += other.total_append_batches;
        self.metrics.total_nullify_batches += other.total_nullify_batches;

        if let Some(min) = other.min_iteration {
            self.metrics.min_iteration = Some(
                self.metrics
                    .min_iteration
                    .map(|m| m.min(min))
                    .unwrap_or(min),
            );
        }
        if let Some(max) = other.max_iteration {
            self.metrics.max_iteration = Some(
                self.metrics
                    .max_iteration
                    .map(|m| m.max(max))
                    .unwrap_or(max),
            );
        }
    }

    pub fn print_performance_summary(&self, label: &str) {
        self.metrics.print_summary(label);
    }
}

pub type SharedState = Arc<RwLock<SharedTreeState>>;

pub fn create_shared_state(initial_root: [u8; 32]) -> SharedState {
    Arc::new(RwLock::new(SharedTreeState::new(initial_root)))
}

#[derive(Debug, Clone, Default)]
pub struct IterationMetrics {
    pub phase1_duration: Duration,
    pub phase2_duration: Duration,
    pub phase3_duration: Duration,
    pub total_duration: Duration,
    pub append_batches: usize,
    pub nullify_batches: usize,
}

impl IterationMetrics {
    pub fn total_batches(&self) -> usize {
        self.append_batches + self.nullify_batches
    }
}

#[derive(Debug, Clone, Default)]
pub struct CumulativeMetrics {
    pub iterations: usize,
    pub total_duration: Duration,
    pub phase1_total: Duration,
    pub phase2_total: Duration,
    pub phase3_total: Duration,
    pub total_append_batches: usize,
    pub total_nullify_batches: usize,
    pub min_iteration: Option<Duration>,
    pub max_iteration: Option<Duration>,
}

impl CumulativeMetrics {
    pub fn add_iteration(&mut self, metrics: &IterationMetrics) {
        self.iterations += 1;
        self.total_duration += metrics.total_duration;
        self.phase1_total += metrics.phase1_duration;
        self.phase2_total += metrics.phase2_duration;
        self.phase3_total += metrics.phase3_duration;
        self.total_append_batches += metrics.append_batches;
        self.total_nullify_batches += metrics.nullify_batches;

        self.min_iteration = Some(
            self.min_iteration
                .map(|min| min.min(metrics.total_duration))
                .unwrap_or(metrics.total_duration),
        );

        self.max_iteration = Some(
            self.max_iteration
                .map(|max| max.max(metrics.total_duration))
                .unwrap_or(metrics.total_duration),
        );
    }

    pub fn avg_iteration_duration(&self) -> Duration {
        if self.iterations > 0 {
            self.total_duration / self.iterations as u32
        } else {
            Duration::ZERO
        }
    }

    pub fn avg_speedup(&self) -> f64 {
        let sequential_estimate = self.phase1_total + self.phase2_total + self.phase3_total;
        if self.total_duration.as_secs_f64() > 0.0 {
            sequential_estimate.as_secs_f64() / self.total_duration.as_secs_f64()
        } else {
            1.0
        }
    }

    pub fn total_batches(&self) -> usize {
        self.total_append_batches + self.total_nullify_batches
    }

    pub fn print_summary(&self, label: &str) {
        println!("\n========================================");
        if !label.is_empty() {
            println!("  {}  ", label.to_uppercase());
            println!("========================================");
        }
        println!("Total iterations:        {}", self.iterations);
        println!("Total duration:          {:?}", self.total_duration);
        println!(
            "Avg iteration:           {:?}",
            self.avg_iteration_duration()
        );

        if let Some(min) = self.min_iteration {
            println!("Min iteration:           {:?}", min);
        }
        if let Some(max) = self.max_iteration {
            println!("Max iteration:           {:?}", max);
        }

        println!();
        println!("Total batches processed:");
        println!("  Append:                {}", self.total_append_batches);
        println!("  Nullify:               {}", self.total_nullify_batches);
        println!("  Total:                 {}", self.total_batches());

        if self.iterations > 0 {
            let avg_phase1 = self.phase1_total / self.iterations as u32;
            let avg_phase2 = self.phase2_total / self.iterations as u32;
            let avg_phase3 = self.phase3_total / self.iterations as u32;

            println!();
            println!("Phase timing breakdown (total / avg per iteration):");
            println!(
                "  Phase 1 (prep):        {:?} / {:?}",
                self.phase1_total, avg_phase1
            );
            println!(
                "  Phase 2 (proof):       {:?} / {:?}",
                self.phase2_total, avg_phase2
            );
            println!(
                "  Phase 3 (submit):      {:?} / {:?}",
                self.phase3_total, avg_phase3
            );
            println!("  ─────────────────────────────────────────────────────");
            println!(
                "  Total (actual):        {:?} / {:?}",
                self.total_duration,
                self.avg_iteration_duration()
            );
            println!();
            println!("Note: Phase 2 and Phase 3 run concurrently (pipelined),");
            println!("      so total < sum of individual phases.");
        }

        println!("========================================\n");
    }
}
