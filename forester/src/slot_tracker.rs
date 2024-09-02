use forester_utils::rpc::RpcConnection;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::UNIX_EPOCH;
use std::{sync::Arc, time::SystemTime};
use tokio::time::{sleep, Duration};
use tracing::{debug, error};

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
        let old_slot = self.last_known_slot.load(Ordering::Acquire);
        self.last_known_slot.store(new_slot, Ordering::Release);
        self.last_update_time.store(now, Ordering::Release);
        debug!(
            "SlotTracker updated: old_slot={}, new_slot={}",
            old_slot, new_slot
        );
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
        debug!(
            "Estimated current slot: {} (last known: {}, elapsed: {:?})",
            estimated_slot, last_slot, elapsed
        );
        estimated_slot
    }

    pub async fn run<R: RpcConnection + Send + 'static>(self: Arc<Self>, rpc: &mut R) {
        loop {
            match rpc.get_slot().await {
                Ok(slot) => {
                    self.update(slot);
                    debug!("SlotTracker run: Updated slot to {}", slot);
                }
                Err(e) => error!("Failed to get slot: {:?}", e),
            }
            tokio::time::sleep(self.update_interval).await;
        }
    }
}

pub async fn wait_until_slot_reached<R: RpcConnection>(
    rpc: &mut R,
    slot_tracker: &Arc<SlotTracker>,
    target_slot: u64,
) -> crate::Result<()> {
    debug!("Waiting for slot {}", target_slot);

    loop {
        let current_estimated_slot = slot_tracker.estimated_current_slot();

        if current_estimated_slot >= target_slot {
            // Double-check with actual RPC call
            let actual_slot = rpc.get_slot().await?;
            if actual_slot >= target_slot {
                break;
            }
        }

        let sleep_duration = if current_estimated_slot < target_slot {
            let slots_to_wait = target_slot - current_estimated_slot;
            Duration::from_secs_f64(slots_to_wait as f64 * slot_duration().as_secs_f64())
        } else {
            slot_duration()
        };

        sleep(sleep_duration).await;
    }

    debug!("Slot {} reached", target_slot);
    Ok(())
}
