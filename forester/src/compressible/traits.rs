//! Shared traits for compressible account tracking.

use std::{
    collections::HashSet,
    fmt,
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
use tracing::info;

use crate::{
    priority_fee::PriorityFeeConfig,
    smart_transaction::{send_smart_transaction, ComputeBudgetConfig, SendSmartTransactionConfig},
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

#[derive(Debug, Clone, Copy, Default)]
pub struct CompressibleTransactionConfig {
    pub priority_fee_config: PriorityFeeConfig,
    pub compute_unit_limit: Option<u32>,
}

fn collect_priority_fee_accounts(payer: Pubkey, instructions: &[Instruction]) -> Vec<Pubkey> {
    let mut seen = HashSet::with_capacity(1 + instructions.len() * 4);
    let mut account_keys = Vec::with_capacity(1 + instructions.len() * 4);

    if seen.insert(payer) {
        account_keys.push(payer);
    }

    for instruction in instructions {
        for account_meta in &instruction.accounts {
            if seen.insert(account_meta.pubkey) {
                account_keys.push(account_meta.pubkey);
            }
        }
    }

    account_keys
}

pub async fn send_with_transaction_policy(
    rpc: &mut impl Rpc,
    instructions: &[Instruction],
    payer: &Keypair,
    transaction_config: CompressibleTransactionConfig,
) -> Result<Signature> {
    let payer_pubkey = payer.pubkey();
    let priority_fee = transaction_config
        .priority_fee_config
        .resolve(
            &*rpc,
            collect_priority_fee_accounts(payer_pubkey, instructions),
        )
        .await?;
    let signers = [payer];

    send_smart_transaction(
        rpc,
        SendSmartTransactionConfig {
            instructions: instructions.to_vec(),
            payer: &payer_pubkey,
            signers: &signers,
            address_lookup_tables: &[],
            compute_budget: ComputeBudgetConfig {
                compute_unit_price: priority_fee,
                compute_unit_limit: transaction_config.compute_unit_limit,
            },
            confirmation: None,
        },
    )
    .await
    .map_err(Into::into)
}

/// Marks `pubkeys` as pending, sends the transaction through the shared
/// transaction policy, and either marks accounts as compressed or unmarks
/// pending on any failure.
pub async fn send_and_confirm_with_tracking<S: CompressibleState>(
    rpc: &mut impl Rpc,
    instructions: &[Instruction],
    payer: &Keypair,
    transaction_config: CompressibleTransactionConfig,
    tracker: &impl CompressibleTracker<S>,
    pubkeys: &[Pubkey],
    tx_label: &str,
) -> Result<Signature> {
    tracker.mark_pending(pubkeys);

    let signature =
        match send_with_transaction_policy(rpc, instructions, payer, transaction_config).await {
            Ok(sig) => sig,
            Err(e) => {
                tracker.unmark_pending(pubkeys);
                return Err(anyhow::anyhow!(
                    "Failed to send or confirm {} transaction: {:?}",
                    tx_label,
                    e
                ));
            }
        };

    info!("{} tx sent: {}", tx_label, signature);

    for pubkey in pubkeys {
        tracker.remove_compressed(pubkey);
    }
    info!("{} tx confirmed: {}", tx_label, signature);
    Ok(signature)
}
