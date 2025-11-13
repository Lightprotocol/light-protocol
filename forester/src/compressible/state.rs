use std::sync::Arc;

use borsh::BorshDeserialize;
use dashmap::DashMap;
use light_ctoken_types::{
    state::{extensions::ExtensionStruct, CToken},
    COMPRESSIBLE_TOKEN_ACCOUNT_SIZE, COMPRESSIBLE_TOKEN_RENT_EXEMPTION,
};
use solana_sdk::pubkey::Pubkey;
use tracing::debug;

use super::types::CompressibleAccountState;
use crate::Result;

/// Calculate the slot at which an account becomes compressible
/// Returns the last funded slot; accounts are compressible when current_slot > this value
fn calculate_compressible_slot(account: &CToken, lamports: u64) -> u64 {
    use light_compressible::rent::SLOTS_PER_EPOCH;

    // Find the Compressible extension
    if let Some(ExtensionStruct::Compressible(compressible_ext)) =
        account.extensions.as_ref().and_then(|exts| {
            exts.iter()
                .find(|ext| matches!(ext, ExtensionStruct::Compressible(_)))
        })
    {
        // Calculate last funded epoch
        let last_funded_epoch = compressible_ext
            .get_last_funded_epoch(
                COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
                lamports,
                COMPRESSIBLE_TOKEN_RENT_EXEMPTION,
            )
            .unwrap_or(0);

        // Convert to slot
        last_funded_epoch * SLOTS_PER_EPOCH
    } else {
        // No compressible extension - return max slot (never compressible)
        u64::MAX
    }
}

/// Tracker for compressible CToken accounts
#[derive(Debug)]
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
                state.account.extensions.as_ref().is_some_and(|exts| {
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
                // Account is compressible if current slot is past the compressible slot
                state.compressible_slot < current_slot
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

        // Calculate compressible slot
        let compressible_slot = calculate_compressible_slot(&ctoken, lamports);

        // Create state with full CToken account
        let state = CompressibleAccountState {
            pubkey,
            account: ctoken,
            lamports,
            compressible_slot,
        };

        debug!(
            "Updated account {}: mint={:?}, owner={:?}, amount={}, compressible_slot={}",
            pubkey,
            state.account.mint,
            state.account.owner,
            state.account.amount,
            compressible_slot
        );

        // Store in DashMap
        self.insert(state);

        Ok(())
    }

    /// Query accounts and update tracker: remove non-existent accounts, update lamports for existing ones
    pub async fn sync_accounts<R: light_client::rpc::Rpc>(
        &self,
        rpc: &R,
        pubkeys: &[Pubkey],
    ) -> Result<()> {
        // Query all accounts at once using get_multiple_accounts
        let accounts = rpc.get_multiple_accounts(pubkeys).await?;

        for (pubkey, account_opt) in pubkeys.iter().zip(accounts.iter()) {
            match account_opt {
                Some(account) => {
                    // Account exists - update lamports and recalculate compressible_slot
                    if let Some(mut state) = self.accounts.get_mut(pubkey) {
                        state.lamports = account.lamports;
                        // Recalculate compressible_slot since lamports changed
                        state.compressible_slot =
                            calculate_compressible_slot(&state.account, account.lamports);
                        debug!(
                            "Updated lamports for account {}: {}, compressible_slot: {}",
                            pubkey, account.lamports, state.compressible_slot
                        );
                    }
                }
                None => {
                    // Account doesn't exist - remove from tracker
                    self.remove(pubkey);
                    debug!("Removed non-existent account {}", pubkey);
                }
            }
        }
        Ok(())
    }
}

impl Default for CompressibleAccountTracker {
    fn default() -> Self {
        Self::new()
    }
}
