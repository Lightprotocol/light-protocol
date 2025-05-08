use std::{collections::HashMap, time::Duration};

use tokio::time::Instant;

#[derive(Debug, Clone)]
pub struct ProcessedHashCache {
    entries: HashMap<String, Instant>,
    ttl: Duration,
}

impl ProcessedHashCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            entries: HashMap::new(),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    pub fn add(&mut self, hash: &str) {
        self.entries.insert(hash.to_string(), Instant::now());
    }

    pub fn contains(&mut self, hash: &str) -> bool {
        self.cleanup();
        self.entries.contains_key(hash)
    }

    pub fn cleanup(&mut self) {
        let now = Instant::now();
        self.entries
            .retain(|_, timestamp| now.duration_since(*timestamp) < self.ttl);
    }
}
