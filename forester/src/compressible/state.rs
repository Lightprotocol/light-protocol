use borsh::BorshDeserialize;
use dashmap::DashMap;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tracing::debug;

use light_compressible::rent::AccountRentState;
use light_ctoken_types::{
    state::{extensions::ExtensionStruct, CToken},
    COMPRESSIBLE_TOKEN_ACCOUNT_SIZE, COMPRESSIBLE_TOKEN_RENT_EXEMPTION,
};

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

    /// Get all accounts with compressible extension
    pub fn get_compressible_accounts(&self) -> Vec<CompressibleAccountState> {
        self.accounts
            .iter()
            .filter(|entry| {
                let state = entry.value();
                // Check if account has compressible extension
                state.account.extensions.as_ref().map_or(false, |exts| {
                    exts.iter()
                        .any(|ext| matches!(ext, ExtensionStruct::Compressible(_)))
                })
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get accounts that are ready to be compressed (rent expired)
    pub fn get_ready_to_compress(&self, current_slot: u64) -> Vec<CompressibleAccountState> {
        self.accounts
            .iter()
            .filter(|entry| {
                let state = entry.value();
                // Check if account has compressible extension and is currently compressible
                if let Some(ExtensionStruct::Compressible(compressible_ext)) =
                    state.account.extensions.as_ref().and_then(|exts| {
                        exts.iter()
                            .find(|ext| matches!(ext, ExtensionStruct::Compressible(_)))
                    })
                {
                    let account_state = AccountRentState {
                        num_bytes: COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as u64,
                        current_slot,
                        current_lamports: state.lamports,
                        last_claimed_slot: compressible_ext.last_claimed_slot,
                    };
                    account_state
                        .is_compressible(
                            &compressible_ext.rent_config,
                            COMPRESSIBLE_TOKEN_RENT_EXEMPTION,
                        )
                        .is_some()
                } else {
                    false
                }
            })
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
    ) -> Result<()> {
        // Deserialize CToken using borsh
        let ctoken = CToken::try_from_slice(account_data)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize CToken with borsh: {:?}", e))?;

        // Create state with full CToken account
        let state = CompressibleAccountState {
            pubkey,
            account: ctoken,
            lamports,
        };

        debug!(
            "Updated account {}: mint={:?}, owner={:?}, amount={}",
            pubkey, state.account.mint, state.account.owner, state.account.amount
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
