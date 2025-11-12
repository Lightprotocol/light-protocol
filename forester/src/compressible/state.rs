use dashmap::DashMap;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tracing::debug;

use light_compressible::rent::AccountRentState;
use light_ctoken_types::{
    state::{CToken, ZExtensionStruct},
    COMPRESSIBLE_TOKEN_RENT_EXEMPTION,
};
use light_zero_copy::traits::ZeroCopyAt;

use super::types::CompressibleAccountState;
use crate::Result;

/// Tracker for compressible CToken accounts
pub struct CompressibleAccountTracker {
    accounts: Arc<DashMap<Pubkey, CompressibleAccountState>>,
}

impl CompressibleAccountTracker {
    /// Create a new tracker
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(DashMap::new()),
        }
    }

    /// Insert or update an account state
    pub fn insert(&self, state: CompressibleAccountState) {
        self.accounts.insert(state.pubkey, state);
    }

    /// Remove an account from tracking
    pub fn remove(&self, pubkey: &Pubkey) -> Option<CompressibleAccountState> {
        self.accounts.remove(pubkey).map(|(_, v)| v)
    }

    /// Get all compressible accounts (accounts where is_compressible == true)
    pub fn get_compressible_accounts(&self) -> Vec<CompressibleAccountState> {
        self.accounts
            .iter()
            .filter(|entry| entry.value().is_compressible)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get total number of tracked accounts
    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    /// Check if tracker is empty
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    /// Update account state from raw account data
    pub fn update_from_account(
        &self,
        pubkey: Pubkey,
        account_data: &[u8],
        lamports: u64,
        slot: u64,
    ) -> Result<()> {
        // Deserialize CToken using zero-copy
        let (ctoken, _) = CToken::zero_copy_at(account_data)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize CToken: {:?}", e))?;

        // Find Compressible extension
        let extensions = ctoken
            .extensions
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No extensions found"))?;

        let compressible_ext = extensions
            .iter()
            .find_map(|ext| match ext {
                ZExtensionStruct::Compressible(e) => Some(e),
                _ => None,
            })
            .ok_or_else(|| anyhow::anyhow!("No compressible extension found"))?;

        // Check if account is compressible using AccountRentState
        let account_state = AccountRentState {
            num_bytes: account_data.len() as u64,
            current_slot: slot,
            current_lamports: lamports,
            last_claimed_slot: u64::from(compressible_ext.last_claimed_slot),
        };

        let is_compressible = account_state
            .is_compressible(
                &compressible_ext.rent_config,
                COMPRESSIBLE_TOKEN_RENT_EXEMPTION,
            )
            .is_some();

        // Extract all required fields
        let state = CompressibleAccountState {
            pubkey,
            mint: Pubkey::from(ctoken.mint.to_bytes()),
            owner: Pubkey::from(ctoken.owner.to_bytes()),
            balance: u64::from(*ctoken.amount),
            last_claimed_slot: u64::from(compressible_ext.last_claimed_slot),
            compression_authority: Pubkey::from(compressible_ext.compression_authority),
            rent_sponsor: Pubkey::from(compressible_ext.rent_sponsor),
            compress_to_pubkey: compressible_ext.compress_to_pubkey(),
            last_seen_slot: slot,
            is_compressible,
        };

        debug!(
            "Updated account {}: compressible={}, balance={}",
            pubkey, is_compressible, state.balance
        );

        // Store in DashMap
        self.insert(state);

        Ok(())
    }
}

impl Default for CompressibleAccountTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_insert_and_remove() {
        let tracker = CompressibleAccountTracker::new();
        let pubkey = Pubkey::new_unique();

        let state = CompressibleAccountState {
            pubkey,
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            balance: 100,
            last_claimed_slot: 0,
            compression_authority: Pubkey::new_unique(),
            rent_sponsor: Pubkey::new_unique(),
            compress_to_pubkey: false,
            last_seen_slot: 1000,
            is_compressible: true,
        };

        tracker.insert(state.clone());
        assert_eq!(tracker.len(), 1);

        let removed = tracker.remove(&pubkey);
        assert!(removed.is_some());
        assert_eq!(tracker.len(), 0);
    }

    #[test]
    fn test_get_compressible_accounts() {
        let tracker = CompressibleAccountTracker::new();

        // Add compressible account
        let compressible_state = CompressibleAccountState {
            pubkey: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            balance: 100,
            last_claimed_slot: 0,
            compression_authority: Pubkey::new_unique(),
            rent_sponsor: Pubkey::new_unique(),
            compress_to_pubkey: false,
            last_seen_slot: 1000,
            is_compressible: true,
        };

        // Add non-compressible account
        let non_compressible_state = CompressibleAccountState {
            pubkey: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            balance: 200,
            last_claimed_slot: 0,
            compression_authority: Pubkey::new_unique(),
            rent_sponsor: Pubkey::new_unique(),
            compress_to_pubkey: false,
            last_seen_slot: 1000,
            is_compressible: false,
        };

        tracker.insert(compressible_state);
        tracker.insert(non_compressible_state);

        assert_eq!(tracker.len(), 2);

        let compressible_accounts = tracker.get_compressible_accounts();
        assert_eq!(compressible_accounts.len(), 1);
        assert!(compressible_accounts[0].is_compressible);
    }
}
