use std::sync::atomic::AtomicU64;

use borsh::BorshDeserialize;
use dashmap::{DashMap, DashSet};
use light_compressible::rent::{
    get_last_funded_epoch, get_rent_exemption_lamports, SLOTS_PER_EPOCH,
};
use light_token_interface::state::{Mint, ACCOUNT_TYPE_MINT};
use solana_sdk::pubkey::Pubkey;
use tracing::{debug, warn};

use super::types::MintAccountState;
use crate::{
    compressible::traits::{CompressibleTracker, SubscriptionHandler},
    Result,
};

pub const ACCOUNT_TYPE_OFFSET: usize = 165;

fn calculate_compressible_slot(mint: &Mint, lamports: u64, account_size: usize) -> Result<u64> {
    let rent_exemption = get_rent_exemption_lamports(account_size as u64)
        .map_err(|e| anyhow::anyhow!("Failed to get rent exemption: {:?}", e))?;
    let compression_info = &mint.compression;
    let config = &compression_info.rent_config;

    let last_funded_epoch = get_last_funded_epoch(
        account_size as u64,
        lamports,
        compression_info.last_claimed_slot,
        config,
        rent_exemption,
    );

    Ok(last_funded_epoch * SLOTS_PER_EPOCH)
}

#[derive(Debug)]
pub struct MintAccountTracker {
    accounts: DashMap<Pubkey, MintAccountState>,
    compressed_count: AtomicU64,
    pending: DashSet<Pubkey>,
}

impl MintAccountTracker {
    pub fn new() -> Self {
        Self {
            accounts: DashMap::new(),
            compressed_count: AtomicU64::new(0),
            pending: DashSet::new(),
        }
    }

    pub fn update_from_account(
        &self,
        pubkey: Pubkey,
        account_data: &[u8],
        lamports: u64,
    ) -> Result<()> {
        if account_data.len() <= ACCOUNT_TYPE_OFFSET {
            debug!("Mint account {} too short, skipping", pubkey);
            return Ok(());
        }

        if account_data[ACCOUNT_TYPE_OFFSET] != ACCOUNT_TYPE_MINT {
            debug!("Account {} is not a Mint, skipping", pubkey);
            return Ok(());
        }

        let mint = match Mint::try_from_slice(account_data) {
            Ok(m) => m,
            Err(e) => {
                debug!("Failed to deserialize Mint {}: {:?}", pubkey, e);
                return Ok(());
            }
        };

        if !mint.metadata.mint_decompressed {
            // Mint was compressed — remove stale entry from tracker if present
            if self.remove(&pubkey).is_some() {
                debug!(
                    "Mint {} no longer decompressed, removed from tracker",
                    pubkey
                );
            }
            return Ok(());
        }

        let expected_mint = Pubkey::new_from_array(mint.metadata.mint.to_bytes());
        if expected_mint != pubkey {
            warn!(
                "Mint PDA mismatch: expected {} but found {}",
                expected_mint, pubkey
            );
            return Ok(());
        }

        let compressible_slot =
            match calculate_compressible_slot(&mint, lamports, account_data.len()) {
                Ok(slot) => slot,
                Err(e) => {
                    warn!(
                        "Failed to calculate compressible slot for {}: {:?}",
                        pubkey, e
                    );
                    return Ok(());
                }
            };

        let mint_seed = Pubkey::new_from_array(mint.metadata.mint_signer);
        let compressed_address = mint.metadata.compressed_address();

        let state = MintAccountState {
            pubkey,
            mint_seed,
            compressed_address,
            mint,
            lamports,
            compressible_slot,
        };

        debug!(
            "Updated Mint {}: mint_seed={}, compressible_slot={}",
            pubkey, mint_seed, compressible_slot
        );

        self.insert(state);
        Ok(())
    }
}

impl CompressibleTracker<MintAccountState> for MintAccountTracker {
    fn accounts(&self) -> &DashMap<Pubkey, MintAccountState> {
        &self.accounts
    }

    fn compressed_counter(&self) -> &AtomicU64 {
        &self.compressed_count
    }

    fn pending(&self) -> &DashSet<Pubkey> {
        &self.pending
    }
}

impl Default for MintAccountTracker {
    fn default() -> Self {
        Self {
            accounts: DashMap::new(),
            compressed_count: AtomicU64::new(0),
            pending: DashSet::new(),
        }
    }
}

impl SubscriptionHandler for MintAccountTracker {
    fn handle_update(
        &self,
        pubkey: Pubkey,
        _program_id: Pubkey,
        data: &[u8],
        lamports: u64,
    ) -> Result<()> {
        // If account data is empty (account was closed), remove from tracker
        if data.is_empty() {
            if self.remove(&pubkey).is_some() {
                debug!("Removed closed Mint account {} from tracker", pubkey);
            }
            return Ok(());
        }
        self.update_from_account(pubkey, data, lamports)
    }

    fn handle_removal(&self, pubkey: &Pubkey) {
        self.remove(pubkey);
    }
}
