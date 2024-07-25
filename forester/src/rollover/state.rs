use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct RolloverState {
    is_rollover_in_progress: AtomicBool,
}

impl RolloverState {
    pub fn new() -> Self {
        Self {
            is_rollover_in_progress: AtomicBool::new(false),
        }
    }

    pub fn try_start_rollover(&self) -> bool {
        self.is_rollover_in_progress
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    pub fn end_rollover(&self) {
        self.is_rollover_in_progress.store(false, Ordering::SeqCst);
    }
}

impl Default for RolloverState {
    fn default() -> Self {
        Self::new()
    }
}
