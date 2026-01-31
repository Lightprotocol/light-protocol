//! LightConfig management for compressible accounts.

use std::collections::HashSet;

use solana_msg::msg;
use solana_pubkey::Pubkey;

use crate::error::LightPdaError;

mod create;
mod state;
mod update;

// --- Constants ---

pub const COMPRESSIBLE_CONFIG_SEED: &[u8] = b"compressible_config";
pub const MAX_ADDRESS_TREES_PER_SPACE: usize = 1;

// Re-export from sdk-types
// --- Re-exports ---
pub use create::{
    check_program_upgrade_authority, process_initialize_light_config,
    process_initialize_light_config_checked,
};
// Re-export Discriminator trait so users can access LightConfig::LIGHT_DISCRIMINATOR
pub use light_account_checks::discriminator::Discriminator;
pub use light_sdk_types::constants::RENT_SPONSOR_SEED;
pub use state::LightConfig;
pub use update::process_update_light_config;

// --- Shared validators (used by create and update) ---

/// Validates that address_space contains no duplicate pubkeys
pub(super) fn validate_address_space_no_duplicates(
    address_space: &[Pubkey],
) -> Result<(), LightPdaError> {
    let mut seen = HashSet::new();
    for pubkey in address_space {
        if !seen.insert(pubkey) {
            msg!("Duplicate pubkey found in address_space: {}", pubkey);
            return Err(LightPdaError::ConstraintViolation);
        }
    }
    Ok(())
}

/// Validates that new_address_space only adds to existing address_space (no removals)
pub(super) fn validate_address_space_only_adds(
    existing_address_space: &[Pubkey],
    new_address_space: &[Pubkey],
) -> Result<(), LightPdaError> {
    for existing_pubkey in existing_address_space {
        if !new_address_space.contains(existing_pubkey) {
            msg!(
                "Cannot remove existing pubkey from address_space: {}",
                existing_pubkey
            );
            return Err(LightPdaError::ConstraintViolation);
        }
    }
    Ok(())
}
