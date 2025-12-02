use std::collections::VecDeque;

use solana_sdk::pubkey::Pubkey;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use super::tx_sender::BatchInstruction;

/// Cached proof ready to be sent
#[derive(Debug, Clone)]
pub struct CachedProof {
    pub seq: u64,
    pub instruction: BatchInstruction,
}

/// Cache for pre-generated proofs.
/// Stores proofs along with the merkle root they were generated against.
/// If the on-chain root changes (another forester processed), the cache is invalidated.
#[derive(Debug)]
pub struct ProofCache {
    /// The merkle tree this cache is for
    tree: Pubkey,
    /// The merkle root at which these proofs were generated
    base_root: [u8; 32],
    /// Pre-generated proofs ready to send
    proofs: VecDeque<CachedProof>,
    /// Whether the cache is currently being populated
    is_warming: bool,
}

impl ProofCache {
    pub fn new(tree: Pubkey) -> Self {
        Self {
            tree,
            base_root: [0u8; 32],
            proofs: VecDeque::new(),
            is_warming: false,
        }
    }

    /// Start warming the cache with a new base root.
    /// Clears any existing cached proofs.
    pub fn start_warming(&mut self, base_root: [u8; 32]) {
        debug!(
            "Starting cache warm-up for tree {} with root {:?}",
            self.tree,
            &base_root[..4]
        );
        self.base_root = base_root;
        self.proofs.clear();
        self.is_warming = true;
    }

    /// Add a pre-generated proof to the cache
    pub fn add_proof(&mut self, seq: u64, instruction: BatchInstruction) {
        if !self.is_warming {
            warn!("Attempted to add proof to cache that is not warming");
            return;
        }
        self.proofs.push_back(CachedProof { seq, instruction });
        debug!(
            "Cached proof seq={} for tree {} (total cached: {})",
            seq,
            self.tree,
            self.proofs.len()
        );
    }

    /// Finish warming the cache
    pub fn finish_warming(&mut self) {
        self.is_warming = false;
        info!(
            "Cache warm-up complete for tree {}: {} proofs cached with root {:?}",
            self.tree,
            self.proofs.len(),
            &self.base_root[..4]
        );
    }

    /// Check if the cache is valid for the given on-chain root.
    /// Returns true if we have cached proofs that match the current root.
    pub fn is_valid_for_root(&self, current_root: &[u8; 32]) -> bool {
        if self.proofs.is_empty() {
            return false;
        }
        if self.is_warming {
            return false;
        }
        &self.base_root == current_root
    }

    /// Take all cached proofs if valid for the given root.
    /// Returns None if the cache is invalid or empty.
    pub fn take_if_valid(&mut self, current_root: &[u8; 32]) -> Option<Vec<CachedProof>> {
        if !self.is_valid_for_root(current_root) {
            if !self.proofs.is_empty() {
                info!(
                    "Cache invalidated for tree {}: expected root {:?}, got {:?}. Discarding {} proofs.",
                    self.tree,
                    &self.base_root[..4],
                    &current_root[..4],
                    self.proofs.len()
                );
            }
            self.proofs.clear();
            return None;
        }

        let proofs: Vec<_> = self.proofs.drain(..).collect();
        info!(
            "Using {} cached proofs for tree {} (root {:?})",
            proofs.len(),
            self.tree,
            &current_root[..4]
        );
        Some(proofs)
    }

    /// Get the number of cached proofs
    pub fn len(&self) -> usize {
        self.proofs.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.proofs.is_empty()
    }

    /// Check if cache is currently warming
    pub fn is_warming(&self) -> bool {
        self.is_warming
    }

    /// Get the base root
    pub fn base_root(&self) -> &[u8; 32] {
        &self.base_root
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.proofs.clear();
        self.is_warming = false;
    }
}

/// Thread-safe wrapper for ProofCache
pub struct SharedProofCache {
    inner: Mutex<ProofCache>,
}

impl std::fmt::Debug for SharedProofCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedProofCache").finish_non_exhaustive()
    }
}

impl SharedProofCache {
    pub fn new(tree: Pubkey) -> Self {
        Self {
            inner: Mutex::new(ProofCache::new(tree)),
        }
    }

    pub async fn start_warming(&self, base_root: [u8; 32]) {
        self.inner.lock().await.start_warming(base_root);
    }

    pub async fn add_proof(&self, seq: u64, instruction: BatchInstruction) {
        self.inner.lock().await.add_proof(seq, instruction);
    }

    pub async fn finish_warming(&self) {
        self.inner.lock().await.finish_warming();
    }

    pub async fn take_if_valid(&self, current_root: &[u8; 32]) -> Option<Vec<CachedProof>> {
        self.inner.lock().await.take_if_valid(current_root)
    }

    pub async fn is_warming(&self) -> bool {
        self.inner.lock().await.is_warming()
    }

    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    pub async fn clear(&self) {
        self.inner.lock().await.clear();
    }
}
