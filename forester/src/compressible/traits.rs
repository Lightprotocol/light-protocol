//! Shared traits for compressible account tracking.

use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::{DashMap, DashSet};
use light_client::rpc::Rpc;
use solana_sdk::{pubkey::Pubkey, signature::Signature};

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

pub async fn verify_transaction_execution(rpc: &impl Rpc, signature: Signature) -> Result<()> {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(500);

    for attempt in 0..MAX_RETRIES {
        let statuses = rpc
            .get_signature_statuses(&[signature])
            .await
            .map_err(|e| {
                anyhow::anyhow!("Failed to get signature status for {}: {:?}", signature, e)
            })?;

        match statuses.first() {
            Some(Some(status)) => {
                if let Some(err) = &status.err {
                    return Err(anyhow::anyhow!(
                        "Transaction {} confirmed but execution failed: {:?}",
                        signature,
                        err
                    ));
                }
                return Ok(());
            }
            _ if attempt < MAX_RETRIES - 1 => {
                tracing::debug!(
                    "Transaction {} status not yet available, retrying ({}/{})",
                    signature,
                    attempt + 1,
                    MAX_RETRIES
                );
                tokio::time::sleep(RETRY_DELAY).await;
            }
            _ => {}
        }
    }

    Err(anyhow::anyhow!(
        "Transaction {} status unavailable after {} retries",
        signature,
        MAX_RETRIES
    ))
}
