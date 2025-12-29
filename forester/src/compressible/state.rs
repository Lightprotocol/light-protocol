use std::sync::Arc;

use borsh::BorshDeserialize;
use dashmap::DashMap;
use light_ctoken_interface::state::CToken;
use solana_sdk::{pubkey::Pubkey, rent::Rent};
use tracing::{debug, warn};

use super::types::CompressibleAccountState;
use crate::Result;

/// Calculate the slot at which an account becomes compressible
/// Returns the last funded slot; accounts are compressible when current_slot > this value
fn calculate_compressible_slot(
    account: &CToken,
    lamports: u64,
    account_size: usize,
) -> Result<u64> {
    use light_compressible::rent::SLOTS_PER_EPOCH;
    use light_ctoken_interface::state::extensions::ExtensionStruct;

    // Calculate rent exemption dynamically
    let rent_exemption = Rent::default().minimum_balance(account_size);

    // Get CompressionInfo from Compressible extension
    let compression_info = account
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|ext| match ext {
                ExtensionStruct::Compressible(comp) => Some(&comp.info),
                _ => None,
            })
        })
        .ok_or_else(|| anyhow::anyhow!("Missing Compressible extension on CToken account"))?;

    // Calculate last funded epoch using embedded compression info
    let last_funded_epoch = compression_info
        .get_last_funded_epoch(account_size as u64, lamports, rent_exemption)
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to calculate last funded epoch for account with {} lamports: {:?}",
                lamports,
                e
            )
        })?;

    // Convert to slot
    Ok(last_funded_epoch * SLOTS_PER_EPOCH)
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

    /// Get all accounts with compressible configuration
    pub fn get_compressible_accounts(&self) -> Vec<CompressibleAccountState> {
        self.accounts
            .iter()
            .filter(|entry| {
                let state = entry.value();
                // Check if account is a valid CToken (account_type == 2)
                state.account.is_ctoken_account()
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
        let compressible_slot =
            match calculate_compressible_slot(&ctoken, lamports, account_data.len()) {
                Ok(slot) => slot,
                Err(e) => {
                    warn!(
                    "Failed to calculate compressible slot for account {}: {}. Skipping account.",
                    pubkey, e
                );
                    return Ok(());
                }
            };

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
}

impl Default for CompressibleAccountTracker {
    fn default() -> Self {
        Self::new()
    }
}
