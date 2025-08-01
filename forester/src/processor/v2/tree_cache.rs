use std::sync::Arc;
use std::collections::HashMap;

use anyhow::Result;
use light_hasher::Hasher;
use light_sparse_merkle_tree::SparseMerkleTree;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Global tree cache instance
pub static TREE_CACHE: tokio::sync::OnceCell<TreeCache> = tokio::sync::OnceCell::const_new();

/// Get or initialize the tree cache
pub async fn get_tree_cache() -> &'static TreeCache {
    TREE_CACHE.get_or_init(|| async { TreeCache::new() }).await
}

/// A snapshot of tree state at a specific point in time
#[derive(Clone, Debug)]
pub struct TreeSnapshot {
    pub subtrees: Vec<[u8; 32]>,
    pub next_index: usize,
    pub root: [u8; 32],
    pub height: usize,
    pub sequence_number: u64,
}

impl TreeSnapshot {
    /// Create a SparseMerkleTree from this snapshot
    pub fn to_tree<H: Hasher, const HEIGHT: usize>(&self) -> Result<SparseMerkleTree<H, HEIGHT>> {
        if self.height != HEIGHT {
            return Err(anyhow::anyhow!(
                "Height mismatch: snapshot has {}, requested {}",
                self.height,
                HEIGHT
            ));
        }
        
        let subtrees_array: [[u8; 32]; HEIGHT] = self.subtrees
            .clone()
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid subtrees length"))?;
        
        Ok(SparseMerkleTree::<H, HEIGHT>::new(subtrees_array, self.next_index))
    }
}

/// Thread-safe cache for merkle tree snapshots
pub struct TreeCache {
    /// Cache of tree snapshots by merkle tree pubkey
    snapshots: Arc<RwLock<HashMap<Pubkey, TreeSnapshot>>>,
    /// Sequence numbers for each tree (for detecting stale data)
    sequences: Arc<RwLock<HashMap<Pubkey, u64>>>,
}

impl TreeCache {
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            sequences: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get a snapshot for a merkle tree
    pub async fn get(&self, merkle_tree: &Pubkey) -> Option<TreeSnapshot> {
        self.snapshots.read().await.get(merkle_tree).cloned()
    }
    
    /// Update or insert a tree snapshot
    pub async fn update<H: Hasher, const HEIGHT: usize>(
        &self,
        merkle_tree: Pubkey,
        tree: &SparseMerkleTree<H, HEIGHT>,
    ) -> Result<()> {
        let mut sequences = self.sequences.write().await;
        let seq = sequences.entry(merkle_tree).or_insert(0);
        *seq += 1;
        let sequence_number = *seq;
        drop(sequences);
        
        let snapshot = TreeSnapshot {
            subtrees: tree.get_subtrees().to_vec(),
            next_index: tree.get_next_index(),
            root: tree.root(),
            height: HEIGHT,
            sequence_number,
        };
        
        let mut snapshots = self.snapshots.write().await;
        snapshots.insert(merkle_tree, snapshot);
        
        debug!("Updated tree cache for {:?} (seq: {})", merkle_tree, sequence_number);
        Ok(())
    }
    
    /// Update from raw tree data
    pub async fn update_from_data(
        &self,
        merkle_tree: Pubkey,
        subtrees: Vec<[u8; 32]>,
        next_index: usize,
        root: [u8; 32],
        height: usize,
    ) -> Result<()> {
        let mut sequences = self.sequences.write().await;
        let seq = sequences.entry(merkle_tree).or_insert(0);
        *seq += 1;
        let sequence_number = *seq;
        drop(sequences);
        
        let snapshot = TreeSnapshot {
            subtrees,
            next_index,
            root,
            height,
            sequence_number,
        };
        
        let mut snapshots = self.snapshots.write().await;
        snapshots.insert(merkle_tree, snapshot);
        
        info!("Updated tree cache from data for {:?} (seq: {})", merkle_tree, sequence_number);
        Ok(())
    }
    
    /// Clear cache for a specific tree
    pub async fn invalidate(&self, merkle_tree: &Pubkey) {
        self.snapshots.write().await.remove(merkle_tree);
        self.sequences.write().await.remove(merkle_tree);
        debug!("Invalidated cache for {:?}", merkle_tree);
    }
    
    /// Clear entire cache
    pub async fn clear(&self) {
        self.snapshots.write().await.clear();
        self.sequences.write().await.clear();
        info!("Cleared entire tree cache");
    }
    
    /// Get or create a tree from cache or data
    pub async fn get_or_create<H: Hasher, const HEIGHT: usize>(
        &self,
        merkle_tree: Pubkey,
        subtrees: &[[u8; 32]],
        next_index: usize,
        root: [u8; 32],
    ) -> Result<SparseMerkleTree<H, HEIGHT>> {
        // Check cache first
        if let Some(snapshot) = self.get(&merkle_tree).await {
            if snapshot.root == root && snapshot.next_index == next_index {
                debug!("Using cached tree for {:?}", merkle_tree);
                return snapshot.to_tree::<H, HEIGHT>();
            }
        }
        
        // Create new tree and cache it
        let subtrees_array: [[u8; 32]; HEIGHT] = subtrees
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid subtrees length"))?;
        
        let tree = SparseMerkleTree::<H, HEIGHT>::new(subtrees_array, next_index);
        
        // Verify root matches
        if tree.root() != root {
            return Err(anyhow::anyhow!("Tree root mismatch after creation"));
        }
        
        // Cache for future use
        self.update(merkle_tree, &tree).await?;
        
        Ok(tree)
    }
}