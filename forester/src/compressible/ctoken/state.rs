use borsh::BorshDeserialize;
use dashmap::DashMap;
use light_compressible::rent::{get_rent_exemption_lamports, SLOTS_PER_EPOCH};
use light_token_interface::state::Token;
use solana_sdk::pubkey::Pubkey;
use tracing::{debug, warn};

use super::types::CTokenAccountState;
use crate::{
    compressible::traits::{CompressibleTracker, SubscriptionHandler},
    Result,
};

fn calculate_compressible_slot(account: &Token, lamports: u64, account_size: usize) -> Result<u64> {
    use light_token_interface::state::extensions::ExtensionStruct;

    let rent_exemption = get_rent_exemption_lamports(account_size as u64)
        .map_err(|e| anyhow::anyhow!("Failed to get rent exemption: {:?}", e))?;

    let compression_info = account
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|ext| match ext {
                ExtensionStruct::Compressible(comp) => Some(&comp.info),
                _ => None,
            })
        })
        .ok_or_else(|| anyhow::anyhow!("Missing Compressible extension on Token account"))?;

    let last_funded_epoch = compression_info
        .get_last_funded_epoch(account_size as u64, lamports, rent_exemption)
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to calculate last funded epoch for account with {} lamports: {:?}",
                lamports,
                e
            )
        })?;

    Ok(last_funded_epoch * SLOTS_PER_EPOCH)
}

#[derive(Debug)]
pub struct CTokenAccountTracker {
    accounts: DashMap<Pubkey, CTokenAccountState>,
}

impl CTokenAccountTracker {
    pub fn new() -> Self {
        Self {
            accounts: DashMap::new(),
        }
    }

    /// Returns all tracked token accounts (not mints), ignoring compressible_slot.
    /// Use `get_ready_to_compress(current_slot)` to get only accounts ready for compression.
    pub fn get_all_token_accounts(&self) -> Vec<CTokenAccountState> {
        self.get_ready_to_compress(u64::MAX)
            .into_iter()
            .filter(|state| state.account.is_token_account())
            .collect()
    }

    pub fn update_from_account(
        &self,
        pubkey: Pubkey,
        account_data: &[u8],
        lamports: u64,
    ) -> Result<()> {
        let ctoken = Token::try_from_slice(account_data)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize Token: {:?}", e))?;

        self.update_from_token(pubkey, ctoken, lamports, account_data.len())
    }

    /// Update tracker with an already-deserialized Token.
    /// Use this to avoid double deserialization when the Token is already available.
    pub fn update_from_token(
        &self,
        pubkey: Pubkey,
        ctoken: Token,
        lamports: u64,
        account_size: usize,
    ) -> Result<()> {
        let compressible_slot = match calculate_compressible_slot(&ctoken, lamports, account_size) {
            Ok(slot) => slot,
            Err(e) => {
                warn!(
                    "Failed to calculate compressible slot for {}: {}",
                    pubkey, e
                );
                return Ok(());
            }
        };

        let state = CTokenAccountState {
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

        self.insert(state);
        Ok(())
    }
}

impl CompressibleTracker<CTokenAccountState> for CTokenAccountTracker {
    fn accounts(&self) -> &DashMap<Pubkey, CTokenAccountState> {
        &self.accounts
    }
}

impl Default for CTokenAccountTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionHandler for CTokenAccountTracker {
    fn handle_update(
        &self,
        pubkey: Pubkey,
        _program_id: Pubkey,
        data: &[u8],
        lamports: u64,
    ) -> Result<()> {
        self.update_from_account(pubkey, data, lamports)
    }

    fn handle_removal(&self, pubkey: &Pubkey) {
        self.remove(pubkey);
    }
}
