use std::sync::Arc;
use std::collections::HashMap;

use anyhow::Result;
use light_sparse_merkle_tree::changelog::ChangelogEntry;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::RwLock;
use tracing::debug;

pub static CHANGELOG_CACHE: tokio::sync::OnceCell<ChangelogCache> = tokio::sync::OnceCell::const_new();

pub async fn get_changelog_cache() -> &'static ChangelogCache {
    CHANGELOG_CACHE.get_or_init(|| async { ChangelogCache::new() }).await
}

pub struct ChangelogCache {
    changelogs: Arc<RwLock<HashMap<Pubkey, Vec<ChangelogEntry<32>>>>>,
}

impl ChangelogCache {
    pub fn new() -> Self {
        Self {
            changelogs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn get_changelogs(&self, merkle_tree: &Pubkey) -> Vec<ChangelogEntry<32>> {
        self.changelogs.read().await.get(merkle_tree).cloned().unwrap_or_default()
    }
    
    
    pub async fn append_changelogs(
        &self,
        merkle_tree: Pubkey,
        new_changelogs: Vec<ChangelogEntry<32>>,
    ) -> Result<()> {
        let mut changelogs = self.changelogs.write().await;
        let entries = changelogs.entry(merkle_tree).or_insert_with(Vec::new);
        let count = new_changelogs.len();
        entries.extend(new_changelogs);
        
        debug!("Appended {} changelogs for {:?}, total entries: {}", 
            count, merkle_tree, entries.len());
        Ok(())
    }
    
    
}