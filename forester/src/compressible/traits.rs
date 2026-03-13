//! Shared traits for compressible account tracking.

use std::{
    fmt,
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

use dashmap::{DashMap, DashSet};
use light_client::rpc::Rpc;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use thiserror::Error;
use tracing::info;

use crate::{
    smart_transaction::{
        collect_priority_fee_accounts, send_transaction_with_policy,
        SendTransactionWithPolicyConfig, TransactionPolicy,
    },
    Result,
};

/// Typed error for compressor cancellation, used instead of string matching.
#[derive(Debug)]
pub struct Cancelled;

impl fmt::Display for Cancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cancelled")
    }
}

impl std::error::Error for Cancelled {}

#[derive(Debug, Error)]
pub enum CompressionTaskError {
    #[error(transparent)]
    Failed(#[from] anyhow::Error),

    #[error("Cancelled")]
    Cancelled,
}

#[derive(Debug)]
pub enum CompressionOutcome<S> {
    Compressed {
        signature: Signature,
        state: S,
    },
    Failed {
        state: S,
        error: CompressionTaskError,
    },
}

pub type CompressionOutcomes<S> = Vec<CompressionOutcome<S>>;

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

struct PendingAccountsGuard<'a, T, S>
where
    T: CompressibleTracker<S> + ?Sized,
    S: CompressibleState,
{
    tracker: &'a T,
    pubkeys: &'a [Pubkey],
    completed: bool,
    state: PhantomData<S>,
}

impl<'a, T, S> PendingAccountsGuard<'a, T, S>
where
    T: CompressibleTracker<S> + ?Sized,
    S: CompressibleState,
{
    fn new(tracker: &'a T, pubkeys: &'a [Pubkey]) -> Self {
        tracker.mark_pending(pubkeys);
        Self {
            tracker,
            pubkeys,
            completed: false,
            state: PhantomData,
        }
    }

    fn complete(mut self) {
        for pubkey in self.pubkeys {
            self.tracker.remove_compressed(pubkey);
        }
        self.completed = true;
    }
}

impl<T, S> Drop for PendingAccountsGuard<'_, T, S>
where
    T: CompressibleTracker<S> + ?Sized,
    S: CompressibleState,
{
    fn drop(&mut self) {
        if !self.completed {
            self.tracker.unmark_pending(self.pubkeys);
        }
    }
}

/// Marks `pubkeys` as pending, sends the transaction through the shared
/// transaction policy, and either marks accounts as compressed or unmarks
/// pending on any failure.
pub async fn send_and_confirm_with_tracking<S: CompressibleState>(
    rpc: &mut impl Rpc,
    instructions: &[Instruction],
    payer: &Keypair,
    transaction_policy: TransactionPolicy,
    tracker: &impl CompressibleTracker<S>,
    pubkeys: &[Pubkey],
    tx_label: &str,
) -> Result<Signature> {
    if transaction_policy.confirmation.is_none() {
        return Err(anyhow::anyhow!(
            "{} transaction requires confirmation to update tracker state safely",
            tx_label
        ));
    }

    let pending = PendingAccountsGuard::<_, S>::new(tracker, pubkeys);
    let payer_pubkey = payer.pubkey();
    let signers = [payer];
    let signature = send_transaction_with_policy(
        rpc,
        SendTransactionWithPolicyConfig {
            instructions: instructions.to_vec(),
            payer: &payer_pubkey,
            signers: &signers,
            address_lookup_tables: &[],
            priority_fee_accounts: collect_priority_fee_accounts(payer_pubkey, instructions),
            policy: transaction_policy,
            confirmation_deadline: None,
        },
    )
    .await
    .map_err(|e| {
        anyhow::anyhow!(
            "Failed to send or confirm {} transaction: {:?}",
            tx_label,
            e
        )
    })?;

    info!("{} tx sent: {}", tx_label, signature);
    pending.complete();
    info!("{} tx confirmed: {}", tx_label, signature);
    Ok(signature)
}
