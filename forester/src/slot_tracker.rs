use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use light_client::rpc::Rpc;
use tokio::time::{sleep, Duration};
use tracing::{error, trace};

pub fn slot_duration() -> Duration {
    Duration::from_nanos(solana_sdk::genesis_config::GenesisConfig::default().ns_per_slot() as u64)
}

#[derive(Debug)]
pub struct SlotTracker {
    last_known_slot: AtomicU64,
    last_update_time: AtomicU64,
    update_interval: Duration,
}

impl SlotTracker {
    pub fn new(initial_slot: u64, update_interval: Duration) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        Self {
            last_known_slot: AtomicU64::new(initial_slot),
            last_update_time: AtomicU64::new(now),
            update_interval,
        }
    }

    pub fn update(&self, new_slot: u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.last_known_slot.store(new_slot, Ordering::Release);
        self.last_update_time.store(now, Ordering::Release);
    }

    pub fn estimated_current_slot(&self) -> u64 {
        let last_slot = self.last_known_slot.load(Ordering::Acquire);
        let last_update = self.last_update_time.load(Ordering::Acquire);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let elapsed = Duration::from_millis(now - last_update);
        let estimated_slot =
            last_slot + (elapsed.as_secs_f64() / slot_duration().as_secs_f64()) as u64;
        trace!(
            "Estimated current slot: {} (last known: {}, elapsed: {:?})",
            estimated_slot,
            last_slot,
            elapsed
        );
        estimated_slot
    }

    pub async fn run<R: Rpc + Send>(self: Arc<Self>, rpc: &mut R) {
        loop {
            match rpc.get_slot().await {
                Ok(slot) => {
                    self.update(slot);
                }
                Err(e) => error!("Failed to get slot: {:?}", e),
            }
            tokio::task::yield_now().await;
            tokio::time::sleep(self.update_interval).await;
        }
    }
}

pub async fn wait_until_slot_reached<R: Rpc>(
    rpc: &mut R,
    slot_tracker: &Arc<SlotTracker>,
    target_slot: u64,
) -> crate::Result<()> {
    trace!("Waiting for slot {}", target_slot);

    const MAX_SLEEP_SLOTS: u64 = 50; // ~20 seconds max sleep between checks

    loop {
        let actual_slot = rpc.get_slot().await?;
        slot_tracker.update(actual_slot);

        if actual_slot >= target_slot {
            trace!("Slot {} reached (actual: {})", target_slot, actual_slot);
            break;
        }

        let slots_remaining = target_slot.saturating_sub(actual_slot);

        let sleep_slots = slots_remaining.min(MAX_SLEEP_SLOTS);
        let sleep_duration =
            Duration::from_secs_f64(sleep_slots as f64 * slot_duration().as_secs_f64());

        trace!(
            "Current slot: {}, target slot: {}, sleeping for {} slots ({:.1} seconds)",
            actual_slot,
            target_slot,
            sleep_slots,
            sleep_duration.as_secs_f64()
        );

        tokio::task::yield_now().await;
        sleep(sleep_duration).await;
    }

    Ok(())
}
