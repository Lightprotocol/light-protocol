use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use light_sparse_merkle_tree::changelog::ChangelogEntry;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::RwLock;
use tracing::{debug, warn};

pub static CHANGELOG_CACHE: tokio::sync::OnceCell<ChangelogCache> =
    tokio::sync::OnceCell::const_new();

pub async fn get_changelog_cache() -> &'static ChangelogCache {
    CHANGELOG_CACHE
        .get_or_init(|| async { ChangelogCache::new() })
        .await
}

struct CacheEntry {
    changelogs: Vec<ChangelogEntry<32>>,
    last_accessed: Instant,
}

pub struct ChangelogCache {
    entries: Arc<RwLock<HashMap<Pubkey, CacheEntry>>>,
    max_entries: usize,
    ttl: Duration,
}

impl ChangelogCache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_entries: 100,              // Default: cache up to 100 trees
            ttl: Duration::from_secs(600), // Default: 10 minute TTL
        }
    }

    pub async fn get_changelogs(&self, merkle_tree: &Pubkey) -> Vec<ChangelogEntry<32>> {
        let mut entries = self.entries.write().await;

        // Check if entry exists and is not expired
        if let Some(entry) = entries.get_mut(merkle_tree) {
            if entry.last_accessed.elapsed() < self.ttl {
                entry.last_accessed = Instant::now();
                return entry.changelogs.clone();
            } else {
                // Entry expired, remove it
                debug!("Removing expired changelog cache for {:?}", merkle_tree);
                entries.remove(merkle_tree);
            }
        }

        Vec::new()
    }

    pub async fn append_changelogs(
        &self,
        merkle_tree: Pubkey,
        new_changelogs: Vec<ChangelogEntry<32>>,
    ) -> Result<()> {
        let mut entries = self.entries.write().await;

        // Evict oldest entries if at capacity
        if entries.len() >= self.max_entries && !entries.contains_key(&merkle_tree) {
            // Find and remove the oldest entry
            if let Some(oldest_key) = entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_accessed)
                .map(|(k, _)| *k)
            {
                warn!("Cache full, evicting oldest entry for {:?}", oldest_key);
                entries.remove(&oldest_key);
            }
        }

        let entry = entries.entry(merkle_tree).or_insert_with(|| CacheEntry {
            changelogs: Vec::new(),
            last_accessed: Instant::now(),
        });

        let count = new_changelogs.len();
        entry.changelogs.extend(new_changelogs);
        entry.last_accessed = Instant::now();

        debug!(
            "Appended {} changelogs for {:?}, total entries: {}",
            count,
            merkle_tree,
            entry.changelogs.len()
        );
        Ok(())
    }
}
