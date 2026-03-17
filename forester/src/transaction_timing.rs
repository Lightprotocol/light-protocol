use std::time::Duration;

use tokio::time::Instant;

use crate::slot_tracker::slot_duration;

const MIN_SCHEDULED_CONFIRMATION_SLOTS: u32 = 4;
const V1_BATCH_TIMEOUT_BUFFER: Duration = Duration::from_secs(2);

fn usable_scheduled_timeout(slots_remaining: u64, buffer: Duration) -> Option<Duration> {
    let slots_remaining = slots_remaining.min(u64::from(u32::MAX)) as u32;
    let timeout = slot_duration()
        .checked_mul(slots_remaining)
        .unwrap_or(Duration::ZERO)
        .saturating_sub(buffer);
    let minimum_timeout = slot_duration()
        .checked_mul(MIN_SCHEDULED_CONFIRMATION_SLOTS)
        .unwrap_or(Duration::ZERO);

    (timeout >= minimum_timeout).then_some(timeout)
}

pub fn scheduled_v1_batch_timeout(slots_remaining: u64) -> Option<Duration> {
    usable_scheduled_timeout(slots_remaining, V1_BATCH_TIMEOUT_BUFFER)
}

pub fn scheduled_confirmation_deadline(slots_remaining: u64) -> Option<Instant> {
    usable_scheduled_timeout(slots_remaining, Duration::ZERO)
        .map(|timeout| Instant::now() + timeout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_confirmation_deadline_requires_minimum_budget() {
        assert!(scheduled_confirmation_deadline(3).is_none());
        assert!(scheduled_confirmation_deadline(4).is_some());
    }

    #[test]
    fn v1_batch_timeout_reserves_headroom() {
        assert_eq!(scheduled_v1_batch_timeout(10), Some(Duration::from_secs(2)));
        assert_eq!(
            scheduled_v1_batch_timeout(9),
            Some(Duration::from_millis(1600))
        );
        assert!(scheduled_v1_batch_timeout(8).is_none());
    }
}
