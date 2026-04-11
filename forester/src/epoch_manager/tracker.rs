use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};

use dashmap::DashMap;
use tokio::sync::Mutex;

use super::ProcessingMetrics;

/// Coordinates re-finalization across parallel `process_queue` tasks when new
/// foresters register mid-epoch. Only one task performs the on-chain
/// `finalize_registration` tx; others wait for it to complete.
#[derive(Debug)]
pub(crate) struct RegistrationTracker {
    cached_registered_weight: AtomicU64,
    refinalize_in_progress: AtomicBool,
    refinalized: tokio::sync::Notify,
}

impl RegistrationTracker {
    pub fn new(weight: u64) -> Self {
        Self {
            cached_registered_weight: AtomicU64::new(weight),
            refinalize_in_progress: AtomicBool::new(false),
            refinalized: tokio::sync::Notify::new(),
        }
    }

    pub fn cached_weight(&self) -> u64 {
        self.cached_registered_weight.load(Ordering::Acquire)
    }

    /// Returns `true` if this caller won the race to perform re-finalization.
    pub fn try_claim_refinalize(&self) -> bool {
        self.refinalize_in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Called by the winner after the on-chain tx succeeds.
    pub fn complete_refinalize(&self, new_weight: u64) {
        self.cached_registered_weight
            .store(new_weight, Ordering::Release);
        self.refinalize_in_progress.store(false, Ordering::Release);
        self.refinalized.notify_waiters();
    }

    /// Called by non-winners to block until re-finalization is done.
    pub async fn wait_for_refinalize(&self) {
        if !self.refinalize_in_progress.load(Ordering::Acquire) {
            return;
        }
        let fut = self.refinalized.notified();
        if !self.refinalize_in_progress.load(Ordering::Acquire) {
            return;
        }
        fut.await;
    }
}

/// Per-epoch coordination state: dedup flags, item counts, metrics, and
/// registration trackers. Shared across parallel tree tasks within an epoch.
#[derive(Debug, Clone)]
pub(crate) struct EpochTracker {
    processing_epochs: Arc<DashMap<u64, Arc<AtomicBool>>>,
    processed_items: Arc<Mutex<HashMap<u64, AtomicUsize>>>,
    processing_metrics: Arc<Mutex<HashMap<u64, ProcessingMetrics>>>,
    registration_trackers: Arc<DashMap<u64, Arc<RegistrationTracker>>>,
}

impl EpochTracker {
    pub fn new() -> Self {
        Self {
            processing_epochs: Arc::new(DashMap::new()),
            processed_items: Arc::new(Mutex::new(HashMap::new())),
            processing_metrics: Arc::new(Mutex::new(HashMap::new())),
            registration_trackers: Arc::new(DashMap::new()),
        }
    }

    /// Attempts to claim exclusive processing for an epoch.
    /// Returns `Some(EpochGuard)` if this caller won, `None` if already claimed.
    /// The guard resets the flag on drop.
    pub fn try_claim_epoch(&self, epoch: u64) -> Option<EpochGuard> {
        let flag = self
            .processing_epochs
            .entry(epoch)
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone();

        if flag
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return None;
        }

        Some(EpochGuard { flag })
    }

    pub async fn get_processed_items_count(&self, epoch: u64) -> usize {
        let counts = self.processed_items.lock().await;
        counts
            .get(&epoch)
            .map_or(0, |count| count.load(Ordering::Relaxed))
    }

    pub async fn increment_processed_items_count(&self, epoch: u64, increment_by: usize) {
        let mut counts = self.processed_items.lock().await;
        counts
            .entry(epoch)
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(increment_by, Ordering::Relaxed);
    }

    pub async fn get_processing_metrics(&self, epoch: u64) -> ProcessingMetrics {
        let metrics = self.processing_metrics.lock().await;
        metrics.get(&epoch).copied().unwrap_or_default()
    }

    pub async fn add_processing_metrics(&self, epoch: u64, new_metrics: ProcessingMetrics) {
        let mut metrics = self.processing_metrics.lock().await;
        *metrics.entry(epoch).or_default() += new_metrics;
    }

    pub fn get_or_create_tracker(&self, epoch: u64, weight: u64) -> Arc<RegistrationTracker> {
        self.registration_trackers
            .entry(epoch)
            .or_insert_with(|| Arc::new(RegistrationTracker::new(weight)))
            .value()
            .clone()
    }

    /// Removes all per-epoch state for the given epoch.
    pub async fn cleanup(&self, epoch: u64) {
        self.registration_trackers.remove(&epoch);
        self.processing_epochs.remove(&epoch);
        self.processed_items.lock().await.remove(&epoch);
        self.processing_metrics.lock().await.remove(&epoch);
    }
}

/// RAII guard that resets the epoch processing flag on drop.
pub(crate) struct EpochGuard {
    flag: Arc<AtomicBool>,
}

impl Drop for EpochGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registration_tracker_initial_weight() {
        let tracker = RegistrationTracker::new(1000);
        assert_eq!(tracker.cached_weight(), 1000);
    }

    #[test]
    fn registration_tracker_claim_returns_true_once() {
        let tracker = RegistrationTracker::new(100);
        assert!(tracker.try_claim_refinalize());
        assert!(!tracker.try_claim_refinalize());
    }

    #[test]
    fn registration_tracker_complete_releases_claim() {
        let tracker = RegistrationTracker::new(100);
        assert!(tracker.try_claim_refinalize());
        tracker.complete_refinalize(200);
        assert_eq!(tracker.cached_weight(), 200);
        // Can claim again after complete
        assert!(tracker.try_claim_refinalize());
    }

    #[tokio::test]
    async fn registration_tracker_wait_returns_immediately_when_not_in_progress() {
        let tracker = RegistrationTracker::new(100);
        // Should return immediately — no one is re-finalizing
        tracker.wait_for_refinalize().await;
    }

    #[tokio::test]
    async fn registration_tracker_wait_blocks_until_complete() {
        let tracker = Arc::new(RegistrationTracker::new(100));
        assert!(tracker.try_claim_refinalize());

        let tracker_clone = tracker.clone();
        let waiter = tokio::spawn(async move {
            tracker_clone.wait_for_refinalize().await;
            tracker_clone.cached_weight()
        });

        // Give the waiter a moment to start waiting
        tokio::task::yield_now().await;

        tracker.complete_refinalize(500);

        let result = waiter.await.unwrap();
        assert_eq!(result, 500);
    }

    #[test]
    fn epoch_tracker_claim_returns_none_on_duplicate() {
        let tracker = EpochTracker::new();
        let guard = tracker.try_claim_epoch(1);
        assert!(guard.is_some());
        assert!(tracker.try_claim_epoch(1).is_none());
        // Different epoch is fine
        assert!(tracker.try_claim_epoch(2).is_some());
    }

    #[test]
    fn epoch_guard_resets_on_drop() {
        let tracker = EpochTracker::new();
        {
            let _guard = tracker.try_claim_epoch(1);
            assert!(tracker.try_claim_epoch(1).is_none());
        }
        // After guard dropped, can claim again
        assert!(tracker.try_claim_epoch(1).is_some());
    }

    #[tokio::test]
    async fn epoch_tracker_items_and_metrics() {
        let tracker = EpochTracker::new();
        assert_eq!(tracker.get_processed_items_count(1).await, 0);

        tracker.increment_processed_items_count(1, 10).await;
        tracker.increment_processed_items_count(1, 5).await;
        assert_eq!(tracker.get_processed_items_count(1).await, 15);

        // Different epoch is independent
        assert_eq!(tracker.get_processed_items_count(2).await, 0);

        let metrics = tracker.get_processing_metrics(1).await;
        assert_eq!(metrics.total(), std::time::Duration::ZERO);

        let new_metrics = ProcessingMetrics {
            tx_sending_duration: std::time::Duration::from_secs(5),
            ..Default::default()
        };
        tracker.add_processing_metrics(1, new_metrics).await;
        let metrics = tracker.get_processing_metrics(1).await;
        assert_eq!(metrics.tx_sending_duration, std::time::Duration::from_secs(5));
    }

    #[tokio::test]
    async fn epoch_tracker_cleanup() {
        let tracker = EpochTracker::new();
        let _guard = tracker.try_claim_epoch(1);
        tracker.increment_processed_items_count(1, 10).await;
        tracker.get_or_create_tracker(1, 100);

        tracker.cleanup(1).await;

        // After cleanup, items and metrics are gone
        assert_eq!(tracker.get_processed_items_count(1).await, 0);
        assert_eq!(tracker.get_processing_metrics(1).await.total(), std::time::Duration::ZERO);
    }
}
