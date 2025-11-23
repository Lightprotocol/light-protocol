use std::collections::VecDeque;

use solana_sdk::pubkey::Pubkey;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use super::tx_sender::BatchInstruction;

#[derive(Debug, Clone)]
pub struct CachedProof {
    pub seq: u64,
    pub old_root: [u8; 32],
    pub new_root: [u8; 32],
    pub instruction: BatchInstruction,
}

#[derive(Debug)]
pub struct ProofCache {
    tree: Pubkey,
    base_root: [u8; 32],
    proofs: VecDeque<CachedProof>,
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

    pub fn add_proof(
        &mut self,
        seq: u64,
        old_root: [u8; 32],
        new_root: [u8; 32],
        instruction: BatchInstruction,
    ) {
        if !self.is_warming {
            warn!("Attempted to add proof to cache that is not warming");
            return;
        }
        if self.proofs.is_empty() && self.base_root != [0u8; 32] && self.base_root != old_root {
            warn!(
                "First cached proof root mismatch for tree {}: base_root={:?}, proof.old_root={:?}",
                self.tree,
                &self.base_root[..4],
                &old_root[..4]
            );
        }

        self.proofs.push_back(CachedProof {
            seq,
            old_root,
            new_root,
            instruction,
        });
        debug!(
            "Cached proof seq={} for tree {} (total cached: {})",
            seq,
            self.tree,
            self.proofs.len()
        );
    }

    pub fn finish_warming(&mut self) {
        self.is_warming = false;
        info!(
            "Cache warm-up complete for tree {}: {} proofs cached with root {:?}",
            self.tree,
            self.proofs.len(),
            &self.base_root[..4]
        );
    }

    pub fn take_if_valid(&mut self, current_root: &[u8; 32]) -> Option<Vec<CachedProof>> {
        if self.proofs.is_empty() || self.is_warming {
            return None;
        }

        let mut skipped = 0;
        while let Some(proof) = self.proofs.front() {
            if proof.old_root == *current_root {
                break;
            }
            if proof.new_root == *current_root {
                self.proofs.pop_front();
                skipped += 1;
                continue;
            }
            self.proofs.pop_front();
            skipped += 1;
        }

        if skipped > 0 {
            debug!(
                "Skipped {} stale cached proofs for tree {} (on-chain already advanced)",
                skipped, self.tree
            );
        }

        if self.proofs.is_empty() {
            debug!(
                "Cache empty after skipping stale proofs for tree {} (current_root {:?})",
                self.tree,
                &current_root[..4]
            );
            return None;
        }

        let mut expected = *current_root;
        let mut taken: Vec<CachedProof> = Vec::new();

        while let Some(proof) = self.proofs.pop_front() {
            if proof.old_root != expected {
                warn!(
                    "Cache chain broken for tree {} at seq {}: expected root {:?}, got {:?}. Dropping remaining {} proofs.",
                    self.tree,
                    proof.seq,
                    &expected[..4],
                    &proof.old_root[..4],
                    self.proofs.len()
                );
                self.proofs.clear();
                break;
            }
            expected = proof.new_root;
            taken.push(proof);
        }

        if taken.is_empty() {
            return None;
        }

        info!(
            "Using {} cached proofs for tree {} starting at root {:?} ending at {:?}{}",
            taken.len(),
            self.tree,
            &current_root[..4],
            &expected[..4],
            if skipped > 0 { format!(" (skipped {} stale)", skipped) } else { String::new() }
        );
        Some(taken)
    }

    pub fn len(&self) -> usize {
        self.proofs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.proofs.is_empty()
    }

    pub fn is_warming(&self) -> bool {
        self.is_warming
    }

    pub fn base_root(&self) -> &[u8; 32] {
        &self.base_root
    }

    pub fn clear(&mut self) {
        self.proofs.clear();
        self.is_warming = false;
    }
}

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

    pub async fn add_proof(
        &self,
        seq: u64,
        old_root: [u8; 32],
        new_root: [u8; 32],
        instruction: BatchInstruction,
    ) {
        self.inner
            .lock()
            .await
            .add_proof(seq, old_root, new_root, instruction);
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
