//! LightConfig management for compressible accounts.

use light_account_checks::AccountInfoTrait;
use light_compressible::rent::RentConfig;

use crate::{
    error::LightPdaError,
    AnchorDeserialize, AnchorSerialize,
};

pub mod create;
mod state;
pub mod update;

// --- Constants ---

pub const COMPRESSIBLE_CONFIG_SEED: &[u8] = b"compressible_config";
pub const MAX_ADDRESS_TREES_PER_SPACE: usize = 1;

// --- Re-exports ---
// Re-export Discriminator trait so users can access LightConfig::LIGHT_DISCRIMINATOR
pub use light_account_checks::discriminator::Discriminator;
pub use light_sdk_types::constants::RENT_SPONSOR_SEED;
pub use state::LightConfig;

// =============================================================================
// Instruction params (serialized by client, deserialized by program)
// =============================================================================

/// Parameters for initialize_compression_config instruction.
/// Uses `[u8; 32]` for pubkeys - borsh-compatible with `solana_pubkey::Pubkey`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeLightConfigParams {
    pub rent_sponsor: [u8; 32],
    pub compression_authority: [u8; 32],
    pub rent_config: RentConfig,
    pub write_top_up: u32,
    pub address_space: Vec<[u8; 32]>,
    pub config_bump: u8,
}

/// Parameters for update_compression_config instruction.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct UpdateLightConfigParams {
    pub new_update_authority: Option<[u8; 32]>,
    pub new_rent_sponsor: Option<[u8; 32]>,
    pub new_compression_authority: Option<[u8; 32]>,
    pub new_rent_config: Option<RentConfig>,
    pub new_write_top_up: Option<u32>,
    pub new_address_space: Option<Vec<[u8; 32]>>,
}

// =============================================================================
// Top-level wrapper functions (remaining_accounts + instruction_data)
// =============================================================================

/// Initialize a LightConfig PDA with upgrade authority check.
///
/// Account layout in remaining_accounts:
/// - [0] payer (signer, mut)
/// - [1] config_account (mut)
/// - [2] program_data_account (readonly)
/// - [3] authority (signer)
/// - [4] system_program
pub fn process_initialize_light_config_checked<AI: AccountInfoTrait + Clone>(
    remaining_accounts: &[AI],
    instruction_data: &[u8],
    program_id: &[u8; 32],
) -> Result<(), LightPdaError> {
    if remaining_accounts.len() < 5 {
        return Err(LightPdaError::NotEnoughAccountKeys);
    }

    let params = InitializeLightConfigParams::try_from_slice(instruction_data)
        .map_err(|_| LightPdaError::Borsh)?;

    create::process_initialize_light_config_checked(
        &remaining_accounts[1], // config_account
        &remaining_accounts[3], // authority
        &remaining_accounts[2], // program_data_account
        &params.rent_sponsor,
        &params.compression_authority,
        params.rent_config,
        params.write_top_up,
        params.address_space,
        params.config_bump,
        &remaining_accounts[0], // payer
        &remaining_accounts[4], // system_program
        program_id,
    )
}

/// Update an existing LightConfig PDA.
///
/// Account layout in remaining_accounts:
/// - [0] config_account (mut)
/// - [1] authority (signer)
pub fn process_update_light_config<AI: AccountInfoTrait + Clone>(
    remaining_accounts: &[AI],
    instruction_data: &[u8],
    program_id: &[u8; 32],
) -> Result<(), LightPdaError> {
    if remaining_accounts.len() < 2 {
        return Err(LightPdaError::NotEnoughAccountKeys);
    }

    let params = UpdateLightConfigParams::try_from_slice(instruction_data)
        .map_err(|_| LightPdaError::Borsh)?;

    update::process_update_light_config(
        &remaining_accounts[0], // config_account
        &remaining_accounts[1], // authority
        params.new_update_authority.as_ref(),
        params.new_rent_sponsor.as_ref(),
        params.new_compression_authority.as_ref(),
        params.new_rent_config,
        params.new_write_top_up,
        params.new_address_space,
        program_id,
    )
}

// --- Shared validators (used by create and update) ---

/// Validates that address_space contains no duplicate pubkeys
pub(super) fn validate_address_space_no_duplicates(
    address_space: &[[u8; 32]],
) -> Result<(), LightPdaError> {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    for pubkey in address_space {
        if !seen.insert(pubkey) {
            return Err(LightPdaError::ConstraintViolation);
        }
    }
    Ok(())
}

/// Validates that new_address_space only adds to existing address_space (no removals)
pub(super) fn validate_address_space_only_adds(
    existing_address_space: &[[u8; 32]],
    new_address_space: &[[u8; 32]],
) -> Result<(), LightPdaError> {
    for existing_pubkey in existing_address_space {
        if !new_address_space.contains(existing_pubkey) {
            return Err(LightPdaError::ConstraintViolation);
        }
    }
    Ok(())
}
