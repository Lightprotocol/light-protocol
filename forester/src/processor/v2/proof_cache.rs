use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use solana_sdk::pubkey::Pubkey;
use tokio::sync::{Mutex, MutexGuard};
use tracing::{debug, info, warn};

use super::tx_sender::BatchInstruction;

const DEFAULT_MAX_CACHED_PROOFS: usize = 256;

#[derive(Debug, Clone)]
pub struct CachedProof {
    pub seq: u64,
    pub old_root: [u8; 32],
    pub new_root: [u8; 32],
    pub instruction: BatchInstruction,
    /// Number of ZKP batch instructions represented by this proof.
    pub items: usize,
}

#[derive(Debug)]
pub struct ProofCache {
    tree: Pubkey,
    base_root: [u8; 32],
    proofs: VecDeque<CachedProof>,
    is_warming: bool,
    warming_generation: u64,
    max_proofs: usize,
}

impl ProofCache {
    pub fn new(tree: Pubkey) -> Self {
        Self {
            tree,
            base_root: [0u8; 32],
            proofs: VecDeque::new(),
            is_warming: false,
            warming_generation: 0,
            max_proofs: DEFAULT_MAX_CACHED_PROOFS,
        }
    }

    pub fn start_warming(&mut self, base_root: [u8; 32]) -> u64 {
        debug!(
            "Starting cache warm-up for tree {} with root {:?}",
            self.tree,
            &base_root[..4]
        );
        self.warming_generation = self.warming_generation.wrapping_add(1);
        self.base_root = base_root;
        // A new collection must not erase completed work from an earlier one.
        self.is_warming = true;
        self.warming_generation
    }

    pub fn add_proof(
        &mut self,
        generation: u64,
        seq: u64,
        old_root: [u8; 32],
        new_root: [u8; 32],
        instruction: BatchInstruction,
    ) {
        if self.warming_generation != generation {
            debug!(
                tree = %self.tree,
                generation,
                active_generation = self.warming_generation,
                "Retaining proof from an older cache collection"
            );
        }
        self.add_late_proof(seq, old_root, new_root, instruction);
    }

    pub fn finish_warming(&mut self, generation: u64) {
        if !self.is_warming || self.warming_generation != generation {
            debug!(
                tree = %self.tree,
                generation,
                active_generation = self.warming_generation,
                "Ignoring completion from an inactive cache warm-up"
            );
            return;
        }

        self.is_warming = false;

        info!(
            "Cache warm-up complete for tree {}: {} proofs cached with root {:?}",
            self.tree,
            self.proofs.len(),
            &self.base_root[..4]
        );
    }

    pub fn add_late_proof(
        &mut self,
        seq: u64,
        old_root: [u8; 32],
        new_root: [u8; 32],
        instruction: BatchInstruction,
    ) {
        let duplicate = self
            .proofs
            .iter()
            .any(|proof| proof.old_root == old_root && proof.new_root == new_root);
        if duplicate {
            debug!(
                tree = %self.tree,
                seq,
                "Ignoring duplicate late proof"
            );
            return;
        }

        let items = instruction.items_count();
        self.proofs.push_back(CachedProof {
            seq,
            old_root,
            new_root,
            instruction,
            items,
        });

        while self.proofs.len() > self.max_proofs {
            if let Some(dropped) = self.proofs.pop_front() {
                warn!(
                    tree = %self.tree,
                    seq = dropped.seq,
                    max = self.max_proofs,
                    "Late proof cache limit reached; dropping oldest candidate"
                );
            }
        }

        debug!(
            tree = %self.tree,
            seq,
            cached_proofs = self.proofs.len(),
            "Cached proof is ready for root-linked reuse"
        );
    }

    /// Snapshot the usable prefix without consuming it. Failed, timed-out or
    /// cancelled sends leave these proofs available until confirmation.
    pub fn ready_chain(&mut self, current_root: &[u8; 32]) -> Option<Vec<CachedProof>> {
        if self.proofs.is_empty() {
            return None;
        }

        // Treat both synchronously warmed and later results as candidates. Proof
        // completion order is not guaranteed, so build the usable chain by roots
        // instead of sequence number. Unmatched candidates stay cached: a missing
        // predecessor may still arrive, or the on-chain root may advance to them.
        let before = self.proofs.len();
        self.proofs.retain(|proof| proof.new_root != *current_root);
        let skipped = before - self.proofs.len();
        let mut candidates = self.proofs.clone();

        if skipped > 0 {
            debug!(
                "Skipped {} stale cached proofs for tree {} (on-chain already advanced)",
                skipped, self.tree
            );
        }

        let mut expected = *current_root;
        let mut taken: Vec<CachedProof> = Vec::new();

        while let Some(position) = candidates
            .iter()
            .position(|proof| proof.old_root == expected)
        {
            let proof = candidates
                .remove(position)
                .expect("candidate position was found");
            expected = proof.new_root;
            taken.push(proof);
        }

        if taken.is_empty() {
            debug!(
                tree = %self.tree,
                current_root = ?&current_root[..4],
                retained_candidates = self.proofs.len(),
                "No cached proof currently links to the on-chain root"
            );
            return None;
        }

        info!(
            "Using {} cached proofs for tree {} starting at root {:?} ending at {:?}{}",
            taken.len(),
            self.tree,
            &current_root[..4],
            &expected[..4],
            if skipped > 0 {
                format!(" (skipped {} stale)", skipped)
            } else {
                String::new()
            }
        );
        Some(taken)
    }

    pub fn len(&self) -> usize {
        self.proofs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.proofs.is_empty()
    }

    pub fn confirm(&mut self, confirmed: &[CachedProof]) {
        self.proofs.retain(|candidate| {
            !confirmed.iter().any(|proof| {
                proof.old_root == candidate.old_root && proof.new_root == candidate.new_root
            })
        });
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

    fn abort_warming(&mut self, generation: u64) {
        if !self.is_warming || self.warming_generation != generation {
            return;
        }

        // Cancellation releases scheduling state, never already completed proofs.
        self.is_warming = false;
        warn!(
            tree = %self.tree,
            generation,
            "Cache warm-up was cancelled; releasing warming state"
        );
    }
}

pub struct SharedProofCache {
    inner: Mutex<ProofCache>,
    sending: Mutex<()>,
    pending_collections: AtomicUsize,
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
            sending: Mutex::new(()),
            pending_collections: AtomicUsize::new(0),
        }
    }

    pub async fn start_warming(self: &Arc<Self>, base_root: [u8; 32]) -> ProofCacheWarmup {
        let generation = self.inner.lock().await.start_warming(base_root);
        ProofCacheWarmup {
            cache: self.clone(),
            generation,
            finished: false,
        }
    }

    pub async fn ready_chain(&self, current_root: &[u8; 32]) -> Option<Vec<CachedProof>> {
        self.inner.lock().await.ready_chain(current_root)
    }

    pub async fn confirm(&self, confirmed: &[CachedProof]) {
        self.inner.lock().await.confirm(confirmed);
    }

    /// Serialize cached sends without blocking proof collection.
    pub fn try_lock_for_sending(&self) -> Option<MutexGuard<'_, ()>> {
        self.sending.try_lock().ok()
    }

    pub fn start_collecting(self: &Arc<Self>) -> ProofCollection {
        self.pending_collections.fetch_add(1, Ordering::AcqRel);
        ProofCollection(self.clone())
    }

    pub async fn has_pending_proofs(&self) -> bool {
        self.pending_collections.load(Ordering::Acquire) > 0 || self.is_warming().await
    }

    pub async fn add_late_proof(
        &self,
        seq: u64,
        old_root: [u8; 32],
        new_root: [u8; 32],
        instruction: BatchInstruction,
    ) {
        self.inner
            .lock()
            .await
            .add_late_proof(seq, old_root, new_root, instruction);
    }

    pub async fn is_warming(&self) -> bool {
        self.inner.lock().await.is_warming()
    }

    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.is_empty()
    }

    pub async fn clear(&self) {
        self.inner.lock().await.clear();
    }
}

/// Bounds speculative work for a tree across the late-result retention period.
/// Dropping or aborting a collector always releases the scheduling gate.
pub struct ProofCollection(Arc<SharedProofCache>);

impl Drop for ProofCollection {
    fn drop(&mut self) {
        self.0.pending_collections.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Owns one cache warm-up session and releases it if its future is cancelled.
///
/// The generation prevents a cancelled, older session from clearing or
/// completing a newer session for the same tree.
pub struct ProofCacheWarmup {
    cache: Arc<SharedProofCache>,
    generation: u64,
    finished: bool,
}

impl ProofCacheWarmup {
    pub async fn add_proof(
        &self,
        seq: u64,
        old_root: [u8; 32],
        new_root: [u8; 32],
        instruction: BatchInstruction,
    ) {
        self.cache.inner.lock().await.add_proof(
            self.generation,
            seq,
            old_root,
            new_root,
            instruction,
        );
    }

    pub async fn finish(mut self) {
        self.cache
            .inner
            .lock()
            .await
            .finish_warming(self.generation);
        self.finished = true;
    }
}

impl Drop for ProofCacheWarmup {
    fn drop(&mut self) {
        if self.finished {
            return;
        }

        if let Ok(mut cache) = self.cache.inner.try_lock() {
            cache.abort_warming(self.generation);
            return;
        }

        let cache = self.cache.clone();
        let generation = self.generation;
        tokio::spawn(async move {
            cache.inner.lock().await.abort_warming(generation);
        });
    }
}

#[cfg(test)]
mod tests {
    use std::future::pending;

    use super::*;

    fn add(cache: &mut ProofCache, seq: u64, old: u8, new: u8) {
        cache.add_late_proof(
            seq,
            [old; 32],
            [new; 32],
            BatchInstruction::Append(Vec::new()),
        );
    }

    #[test]
    fn ready_prefix_is_available_during_collection_and_until_confirmed() {
        let mut cache = ProofCache::new(Pubkey::new_unique());
        let generation = cache.start_warming([1; 32]);
        cache.add_proof(
            generation,
            0,
            [1; 32],
            [2; 32],
            BatchInstruction::Append(Vec::new()),
        );
        let first = cache.ready_chain(&[1; 32]).unwrap();
        assert!(cache.is_warming());
        assert_eq!(first.len(), 1);
        // A failed or timed-out send doesn't consume the prefix.
        assert_eq!(cache.ready_chain(&[1; 32]).unwrap().len(), 1);
        cache.confirm(&first);
        cache.add_proof(
            generation,
            1,
            [2; 32],
            [3; 32],
            BatchInstruction::Append(Vec::new()),
        );
        cache.finish_warming(generation);
        assert_eq!(cache.ready_chain(&[2; 32]).unwrap()[0].new_root, [3; 32]);
        assert!(cache.ready_chain(&[1; 32]).is_none());
    }

    #[test]
    fn new_or_cancelled_warmup_preserves_completed_work() {
        let mut cache = ProofCache::new(Pubkey::new_unique());
        let old = cache.start_warming([1; 32]);
        add(&mut cache, 0, 1, 2);
        let new = cache.start_warming([2; 32]);
        // An older result remains useful even after a new collection starts.
        cache.add_proof(
            old,
            1,
            [2; 32],
            [3; 32],
            BatchInstruction::Append(Vec::new()),
        );
        cache.abort_warming(old);
        assert!(cache.is_warming());
        cache.abort_warming(new);
        assert!(!cache.is_warming());
        assert_eq!(cache.ready_chain(&[1; 32]).unwrap().len(), 2);
        let empty = cache.start_warming([1; 32]);
        cache.finish_warming(empty);
        assert_eq!(cache.ready_chain(&[1; 32]).unwrap().len(), 2);
    }

    #[test]
    fn only_confirmed_prefix_is_removed_after_partial_send() {
        let mut cache = ProofCache::new(Pubkey::new_unique());
        for i in 1..=6 {
            add(&mut cache, i as u64, i, i + 1);
        }
        let chain = cache.ready_chain(&[1; 32]).unwrap();
        cache.confirm(&chain[..4]);
        // The second transaction failed: its two proofs are still retryable.
        let retry = cache.ready_chain(&[5; 32]).unwrap();
        assert_eq!(retry.len(), 2);
        assert_eq!(retry[0].old_root, [5; 32]);
        assert_eq!(retry[1].new_root, [7; 32]);
    }

    #[test]
    fn confirmed_on_chain_proof_is_not_replayed_after_ambiguous_timeout() {
        let mut cache = ProofCache::new(Pubkey::new_unique());
        add(&mut cache, 0, 1, 2);
        add(&mut cache, 1, 2, 3);
        let _unconfirmed_send = cache.ready_chain(&[1; 32]).unwrap();
        let retry = cache.ready_chain(&[2; 32]).unwrap();
        assert_eq!(retry.len(), 1);
        assert_eq!(retry[0].old_root, [2; 32]);
    }

    #[tokio::test]
    async fn cancelled_cached_send_preserves_proofs_and_releases_send_lock() {
        let cache = Arc::new(SharedProofCache::new(Pubkey::new_unique()));
        cache
            .add_late_proof(0, [1; 32], [2; 32], BatchInstruction::Append(Vec::new()))
            .await;
        let send_cache = cache.clone();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _guard = send_cache.try_lock_for_sending().unwrap();
            let _proofs = send_cache.ready_chain(&[1; 32]).await.unwrap();
            started_tx.send(()).unwrap();
            pending::<()>().await;
        });
        started_rx.await.unwrap();
        assert!(cache.try_lock_for_sending().is_none());
        // Receiving more proofs never waits for the sending lock.
        cache
            .add_late_proof(1, [2; 32], [3; 32], BatchInstruction::Append(Vec::new()))
            .await;
        task.abort();
        let _ = task.await;
        assert!(cache.try_lock_for_sending().is_some());
        assert_eq!(cache.ready_chain(&[1; 32]).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn pending_collectors_bound_new_work_without_hiding_ready_proofs() {
        let cache = Arc::new(SharedProofCache::new(Pubkey::new_unique()));
        let first = cache.start_collecting();
        let second = cache.start_collecting();
        let warmup = cache.start_warming([1; 32]).await;
        warmup
            .add_proof(0, [1; 32], [2; 32], BatchInstruction::Append(Vec::new()))
            .await;
        warmup.finish().await;
        assert!(cache.has_pending_proofs().await);
        assert!(cache.ready_chain(&[1; 32]).await.is_some());
        drop(first);
        assert!(cache.has_pending_proofs().await);
        drop(second);
        assert!(!cache.has_pending_proofs().await);
    }

    #[test]
    fn cache_deduplicates_by_roots_and_enforces_one_capacity_limit() {
        let mut cache = ProofCache::new(Pubkey::new_unique());
        cache.max_proofs = 2;
        add(&mut cache, 0, 1, 2);
        add(&mut cache, 99, 1, 2);
        assert_eq!(cache.len(), 1);
        add(&mut cache, 1, 2, 3);
        add(&mut cache, 2, 3, 4);
        assert_eq!(cache.len(), 2);
        assert!(cache.ready_chain(&[1; 32]).is_none());
        assert_eq!(cache.ready_chain(&[2; 32]).unwrap().len(), 2);
    }

    #[tokio::test]
    async fn cancelled_warmup_releases_warming_state() {
        let cache = Arc::new(SharedProofCache::new(Pubkey::new_unique()));
        let task_cache = cache.clone();

        let task = tokio::spawn(async move {
            let _warmup = task_cache.start_warming([1u8; 32]).await;
            pending::<()>().await;
        });

        while !cache.is_warming().await {
            tokio::task::yield_now().await;
        }

        task.abort();
        let _ = task.await;
        tokio::task::yield_now().await;

        assert!(!cache.is_warming().await);
    }

    #[tokio::test]
    async fn cancelled_old_warmup_does_not_clear_new_session() {
        let cache = Arc::new(SharedProofCache::new(Pubkey::new_unique()));
        let old_warmup = cache.start_warming([1u8; 32]).await;
        let new_warmup = cache.start_warming([2u8; 32]).await;

        drop(old_warmup);

        assert!(cache.is_warming().await);
        new_warmup.finish().await;
        assert!(!cache.is_warming().await);
    }

    #[tokio::test]
    async fn late_proofs_are_linked_by_root_when_they_arrive_out_of_order() {
        let cache = Arc::new(SharedProofCache::new(Pubkey::new_unique()));
        let root_1 = [1u8; 32];
        let root_2 = [2u8; 32];
        let root_3 = [3u8; 32];

        cache
            .add_late_proof(1, root_2, root_3, BatchInstruction::Append(Vec::new()))
            .await;
        cache
            .add_late_proof(0, root_1, root_2, BatchInstruction::Append(Vec::new()))
            .await;

        let proofs = cache.ready_chain(&root_1).await.unwrap();
        assert_eq!(proofs.len(), 2);
        assert_eq!(proofs[0].old_root, root_1);
        assert_eq!(proofs[0].new_root, root_2);
        assert_eq!(proofs[1].old_root, root_2);
        assert_eq!(proofs[1].new_root, root_3);
        assert_eq!(cache.len().await, 2);
        cache.confirm(&proofs).await;
        assert!(cache.is_empty().await);
    }
}
