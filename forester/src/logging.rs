use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::{Duration, Instant},
};

#[derive(Debug, Default)]
pub struct ServiceHeartbeat {
    active_cycles: AtomicU64,
    tree_tasks_spawned: AtomicU64,
    queues_started: AtomicU64,
    queues_finished: AtomicU64,
    items_processed: AtomicU64,
    work_reports_sent: AtomicU64,
    v2_recoverable_errors: AtomicU64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct HeartbeatSnapshot {
    pub active_cycles: u64,
    pub tree_tasks_spawned: u64,
    pub queues_started: u64,
    pub queues_finished: u64,
    pub items_processed: u64,
    pub work_reports_sent: u64,
    pub v2_recoverable_errors: u64,
}

impl HeartbeatSnapshot {
    pub fn delta_since(&self, previous: &Self) -> Self {
        Self {
            active_cycles: self.active_cycles.saturating_sub(previous.active_cycles),
            tree_tasks_spawned: self
                .tree_tasks_spawned
                .saturating_sub(previous.tree_tasks_spawned),
            queues_started: self.queues_started.saturating_sub(previous.queues_started),
            queues_finished: self
                .queues_finished
                .saturating_sub(previous.queues_finished),
            items_processed: self
                .items_processed
                .saturating_sub(previous.items_processed),
            work_reports_sent: self
                .work_reports_sent
                .saturating_sub(previous.work_reports_sent),
            v2_recoverable_errors: self
                .v2_recoverable_errors
                .saturating_sub(previous.v2_recoverable_errors),
        }
    }
}

impl ServiceHeartbeat {
    pub fn increment_active_cycle(&self) {
        self.active_cycles.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_tree_tasks_spawned(&self, count: usize) {
        self.tree_tasks_spawned
            .fetch_add(count as u64, Ordering::Relaxed);
    }

    pub fn increment_queue_started(&self) {
        self.queues_started.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_queue_finished(&self) {
        self.queues_finished.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_items_processed(&self, count: usize) {
        self.items_processed
            .fetch_add(count as u64, Ordering::Relaxed);
    }

    pub fn increment_work_report_sent(&self) {
        self.work_reports_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_v2_recoverable_error(&self) {
        self.v2_recoverable_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> HeartbeatSnapshot {
        HeartbeatSnapshot {
            active_cycles: self.active_cycles.load(Ordering::Relaxed),
            tree_tasks_spawned: self.tree_tasks_spawned.load(Ordering::Relaxed),
            queues_started: self.queues_started.load(Ordering::Relaxed),
            queues_finished: self.queues_finished.load(Ordering::Relaxed),
            items_processed: self.items_processed.load(Ordering::Relaxed),
            work_reports_sent: self.work_reports_sent.load(Ordering::Relaxed),
            v2_recoverable_errors: self.v2_recoverable_errors.load(Ordering::Relaxed),
        }
    }
}

pub fn should_emit_rate_limited_warning(key: impl Into<String>, interval: Duration) -> bool {
    static LAST_EMIT_AT: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
    const MAX_KEYS: usize = 2_048;

    let key = key.into();
    let now = Instant::now();
    let map = LAST_EMIT_AT.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map.lock().expect("rate limiter mutex poisoned");

    if map.len() > MAX_KEYS {
        let stale_after = interval.saturating_mul(4);
        map.retain(|_, ts| now.duration_since(*ts) < stale_after);
    }

    match map.get(&key) {
        Some(last_emit) if now.duration_since(*last_emit) < interval => false,
        _ => {
            map.insert(key, now);
            true
        }
    }
}
