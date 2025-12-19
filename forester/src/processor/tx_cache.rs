use std::{collections::HashMap, time::Duration};

use tokio::time::Instant;
use tracing::{trace, warn};

const CLEANUP_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
struct CacheEntry {
    timestamp: Instant,
    timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct ProcessedHashCache {
    entries: HashMap<String, CacheEntry>,
    ttl: Duration,
    last_cleanup: Instant,
}

impl ProcessedHashCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            entries: HashMap::new(),
            ttl: Duration::from_secs(ttl_seconds),
            last_cleanup: Instant::now(),
        }
    }

    pub fn add(&mut self, hash: &str) {
        self.maybe_cleanup();
        self.entries.insert(
            hash.to_string(),
            CacheEntry {
                timestamp: Instant::now(),
                timeout: self.ttl,
            },
        );
    }

    pub fn add_with_timeout(&mut self, hash: &str, timeout: Duration) {
        self.maybe_cleanup();
        self.entries.insert(
            hash.to_string(),
            CacheEntry {
                timestamp: Instant::now(),
                timeout,
            },
        );
    }

    pub fn extend_timeout(&mut self, hash: &str, new_timeout: Duration) {
        if let Some(entry) = self.entries.get_mut(hash) {
            entry.timeout = new_timeout;
        }
    }

    pub fn contains(&mut self, hash: &str) -> bool {
        self.maybe_cleanup();
        if let Some(entry) = self.entries.get(hash) {
            let age = Instant::now().duration_since(entry.timestamp);
            if age > Duration::from_secs(60) && age < entry.timeout {
                trace!(
                    "Cache entry {} has been processing for {:?} (timeout: {:?})",
                    hash,
                    age,
                    entry.timeout
                );
            }
            true
        } else {
            false
        }
    }

    pub fn get_age(&self, hash: &str) -> Option<Duration> {
        self.entries
            .get(hash)
            .map(|entry| Instant::now().duration_since(entry.timestamp))
    }

    #[inline]
    fn maybe_cleanup(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_cleanup) >= CLEANUP_INTERVAL {
            self.cleanup_internal(now);
        }
    }

    fn cleanup_internal(&mut self, now: Instant) {
        self.entries.retain(|hash, entry| {
            let age = now.duration_since(entry.timestamp);
            let should_keep = age < entry.timeout;
            if !should_keep && age < Duration::from_secs(30) {
                warn!(
                    "Removing cache entry {} after {:?} timeout (was: {:?})",
                    hash, age, entry.timeout
                );
            }
            should_keep
        });
        self.last_cleanup = now;
    }

    pub fn cleanup(&mut self) {
        self.cleanup_internal(Instant::now());
    }

    pub fn cleanup_by_key(&mut self, key: &str) {
        self.entries.remove(key);
    }
}
