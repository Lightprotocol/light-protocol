//! Shared traits for compressible account tracking.

use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::{DashMap, DashSet};
use solana_sdk::pubkey::Pubkey;

use crate::Result;

pub trait CompressibleState: Clone + Send + Sync {
    fn pubkey(&self) -> &Pubkey;
    fn lamports(&self) -> u64;
    fn compressible_slot(&self) -> u64;

    fn is_ready_to_compress(&self, current_slot: u64) -> bool {
        current_slot > self.compressible_slot()
    }
}

/// Implementors only need to provide `accounts()`, `compressed_counter()`,
/// and `pending()` — all other methods have default implementations.
pub trait CompressibleTracker<S: CompressibleState>: Send + Sync {
    fn accounts(&self) -> &DashMap<Pubkey, S>;

    /// Counter for total accounts successfully compressed and removed from the tracker.
    fn compressed_counter(&self) -> &AtomicU64;

    /// Set of account pubkeys with in-flight compression transactions.
    /// Accounts in this set are skipped by `get_ready_to_compress()`.
    fn pending(&self) -> &DashSet<Pubkey>;

    fn insert(&self, state: S) {
        self.accounts().insert(*state.pubkey(), state);
    }

    fn remove(&self, pubkey: &Pubkey) -> Option<S> {
        self.pending().remove(pubkey);
        self.accounts().remove(pubkey).map(|(_, v)| v)
    }

    /// Remove an account after successful compression, incrementing the compressed counter.
    fn remove_compressed(&self, pubkey: &Pubkey) -> Option<S> {
        let removed = self.remove(pubkey);
        if removed.is_some() {
            self.compressed_counter().fetch_add(1, Ordering::Relaxed);
        }
        removed
    }

    /// Total number of accounts successfully compressed since startup.
    fn total_compressed(&self) -> u64 {
        self.compressed_counter().load(Ordering::Relaxed)
    }

    /// Mark accounts as pending (in-flight tx). They will be skipped by
    /// `get_ready_to_compress()` until confirmed or returned to the pool.
    fn mark_pending(&self, pubkeys: &[Pubkey]) {
        for pk in pubkeys {
            self.pending().insert(*pk);
        }
    }

    /// Return accounts to the work pool after a failed transaction.
    fn unmark_pending(&self, pubkeys: &[Pubkey]) {
        for pk in pubkeys {
            self.pending().remove(pk);
        }
    }

    fn len(&self) -> usize {
        self.accounts().len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get_ready_to_compress(&self, current_slot: u64) -> Vec<S> {
        let pending = self.pending();
        self.accounts()
            .iter()
            .filter(|entry| {
                entry.value().is_ready_to_compress(current_slot) && !pending.contains(entry.key())
            })
            .map(|entry| entry.value().clone())
            .collect()
    }
}

/// Allows AccountSubscriber to work with any tracker type.
pub trait SubscriptionHandler: Send + Sync {
    fn handle_update(
        &self,
        pubkey: Pubkey,
        program_id: Pubkey,
        data: &[u8],
        lamports: u64,
    ) -> Result<()>;

    fn handle_removal(&self, pubkey: &Pubkey);
}
