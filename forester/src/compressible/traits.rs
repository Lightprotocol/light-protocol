//! Shared traits for compressible account tracking.

use dashmap::DashMap;
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

/// Implementors only need to provide `accounts()` - all other methods have default implementations.
pub trait CompressibleTracker<S: CompressibleState>: Send + Sync {
    fn accounts(&self) -> &DashMap<Pubkey, S>;

    fn insert(&self, state: S) {
        self.accounts().insert(*state.pubkey(), state);
    }

    fn remove(&self, pubkey: &Pubkey) -> Option<S> {
        self.accounts().remove(pubkey).map(|(_, v)| v)
    }

    fn len(&self) -> usize {
        self.accounts().len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get_ready_to_compress(&self, current_slot: u64) -> Vec<S> {
        self.accounts()
            .iter()
            .filter(|entry| entry.value().is_ready_to_compress(current_slot))
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
