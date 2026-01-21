use borsh::BorshDeserialize;
use dashmap::DashMap;
use light_compressible::rent::{get_last_funded_epoch, get_rent_exemption_lamports, SLOTS_PER_EPOCH};
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

    let last_funded_epoch = get_last_funded_epoch(
        account_size as u64,
        lamports,
        compression_info.last_claimed_slot,
        &compression_info.rent_config,
        rent_exemption,
    );

    Ok(last_funded_epoch * SLOTS_PER_EPOCH)
}

#[derive(Debug)]
pub struct MintAccountTracker {
    accounts: DashMap<Pubkey, MintAccountState>,
}

impl MintAccountTracker {
    pub fn new() -> Self {
        Self {
            accounts: DashMap::new(),
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
            debug!("Mint {} is not decompressed, skipping", pubkey);
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
}

impl Default for MintAccountTracker {
    fn default() -> Self {
        Self::new()
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
        self.update_from_account(pubkey, data, lamports)
    }

    fn handle_removal(&self, pubkey: &Pubkey) {
        self.remove(pubkey);
    }
}
