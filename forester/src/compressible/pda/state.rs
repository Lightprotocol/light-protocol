use borsh::BorshDeserialize;
use dashmap::DashMap;
use light_compressible::rent::{get_last_funded_epoch, get_rent_exemption_lamports, SLOTS_PER_EPOCH};
use light_sdk::compressible::compression_info::CompressionInfo;
use solana_sdk::pubkey::Pubkey;
use tracing::{debug, warn};

use super::types::PdaAccountState;
use crate::{
    compressible::{
        config::PdaProgramConfig,
        traits::{CompressibleTracker, SubscriptionHandler},
    },
    Result,
};

/// Layout: [8-byte discriminator][Option<CompressionInfo>][rest of data]
fn extract_compression_info(data: &[u8]) -> Option<CompressionInfo> {
    const DISCRIMINATOR_SIZE: usize = 8;
    if data.len() <= DISCRIMINATOR_SIZE {
        return None;
    }
    Option::<CompressionInfo>::deserialize(&mut &data[DISCRIMINATOR_SIZE..]).ok()?
}

fn calculate_compressible_slot(
    compression_info: &CompressionInfo,
    lamports: u64,
    account_size: usize,
) -> Result<u64> {
    let rent_exemption = get_rent_exemption_lamports(account_size as u64)
        .map_err(|e| anyhow::anyhow!("Failed to get rent exemption: {:?}", e))?;

    let last_funded_epoch = get_last_funded_epoch(
        account_size as u64,
        lamports,
        compression_info.last_claimed_slot(),
        &compression_info.rent_config,
        rent_exemption,
    );

    Ok(last_funded_epoch * SLOTS_PER_EPOCH)
}

#[derive(Debug)]
pub struct PdaAccountTracker {
    accounts: DashMap<Pubkey, PdaAccountState>,
    programs: Vec<PdaProgramConfig>,
}

impl PdaAccountTracker {
    pub fn new(programs: Vec<PdaProgramConfig>) -> Self {
        Self {
            accounts: DashMap::new(),
            programs,
        }
    }

    pub fn programs(&self) -> &[PdaProgramConfig] {
        &self.programs
    }

    pub fn get_ready_to_compress_for_program(
        &self,
        program_id: &Pubkey,
        current_slot: u64,
    ) -> Vec<PdaAccountState> {
        self.get_ready_to_compress(current_slot)
            .into_iter()
            .filter(|state| state.program_id == *program_id)
            .collect()
    }

    pub fn update_from_account(
        &self,
        pubkey: Pubkey,
        program_id: Pubkey,
        account_data: &[u8],
        lamports: u64,
    ) -> Result<()> {
        const DISCRIMINATOR_SIZE: usize = 8;

        if account_data.len() < DISCRIMINATOR_SIZE {
            debug!("Account {} too short, skipping", pubkey);
            return Ok(());
        }

        if let Some(program_config) = self.programs.iter().find(|c| c.program_id == program_id) {
            let account_discriminator: [u8; 8] = account_data[..DISCRIMINATOR_SIZE]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to convert discriminator slice"))?;

            if account_discriminator != program_config.discriminator {
                debug!("Account {} discriminator mismatch, skipping", pubkey);
                return Ok(());
            }
        } else {
            debug!("No config for program {}, skipping {}", program_id, pubkey);
            return Ok(());
        }

        let compression_info = match extract_compression_info(account_data) {
            Some(info) => info,
            None => {
                debug!("Account {} has no CompressionInfo, skipping", pubkey);
                return Ok(());
            }
        };

        if compression_info.is_compressed() {
            debug!(
                "Account {} is already compressed; skipping re-compression",
                pubkey
            );
            return Ok(());
        }

        let compressible_slot =
            match calculate_compressible_slot(&compression_info, lamports, account_data.len()) {
                Ok(slot) => slot,
                Err(e) => {
                    warn!(
                        "Failed to calculate compressible slot for {}: {}",
                        pubkey, e
                    );
                    return Ok(());
                }
            };

        let state = PdaAccountState {
            pubkey,
            program_id,
            lamports,
            compressible_slot,
        };

        debug!(
            "Updated PDA {}: program={}, slot={}",
            pubkey, program_id, compressible_slot
        );

        self.insert(state);
        Ok(())
    }
}

impl CompressibleTracker<PdaAccountState> for PdaAccountTracker {
    fn accounts(&self) -> &DashMap<Pubkey, PdaAccountState> {
        &self.accounts
    }
}

impl Default for PdaAccountTracker {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl SubscriptionHandler for PdaAccountTracker {
    fn handle_update(
        &self,
        pubkey: Pubkey,
        program_id: Pubkey,
        data: &[u8],
        lamports: u64,
    ) -> Result<()> {
        self.update_from_account(pubkey, program_id, data, lamports)
    }

    fn handle_removal(&self, pubkey: &Pubkey) {
        self.remove(pubkey);
    }
}
