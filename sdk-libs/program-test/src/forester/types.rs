//! Registry types and PDA derivations for forester operations.
//!
//! This module provides local copies of registry types to avoid depending on
//! the `light_registry` program crate.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_pubkey::Pubkey;

/// Registry Program ID
pub const REGISTRY_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX");

/// Compressed Token Program ID
pub const COMPRESSED_TOKEN_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m");

// PDA Seeds
pub const FORESTER_SEED: &[u8] = b"forester";
pub const FORESTER_EPOCH_SEED: &[u8] = b"forester_epoch";
pub const PROTOCOL_CONFIG_PDA_SEED: &[u8] = b"authority";

/// Forester configuration
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct ForesterConfig {
    /// Fee in percentage points.
    pub fee: u64,
}

/// Forester PDA account data
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct ForesterPda {
    pub authority: Pubkey,
    pub config: ForesterConfig,
    pub active_weight: u64,
    /// Pending weight which will get active once the next epoch starts.
    pub pending_weight: u64,
    pub current_epoch: u64,
    /// Link to previous compressed forester epoch account hash.
    pub last_compressed_forester_epoch_pda_hash: [u8; 32],
    pub last_registered_epoch: u64,
}

/// Indices for compress and close operation
#[derive(Debug, Copy, Clone, BorshSerialize, BorshDeserialize)]
pub struct CompressAndCloseIndices {
    pub source_index: u8,
    pub mint_index: u8,
    pub owner_index: u8,
    pub rent_sponsor_index: u8,
}

// ============================================================================
// PDA Derivation Functions
// ============================================================================

/// Derives the forester PDA from authority
pub fn get_forester_pda(authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[FORESTER_SEED, authority.as_ref()], &REGISTRY_PROGRAM_ID)
}

/// Derives the forester epoch PDA from forester PDA and epoch
pub fn get_forester_epoch_pda(forester_pda: &Pubkey, epoch: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            FORESTER_EPOCH_SEED,
            forester_pda.as_ref(),
            epoch.to_le_bytes().as_slice(),
        ],
        &REGISTRY_PROGRAM_ID,
    )
}

/// Derives the forester epoch PDA from authority and epoch
pub fn get_forester_epoch_pda_from_authority(authority: &Pubkey, epoch: u64) -> (Pubkey, u8) {
    let forester_pda = get_forester_pda(authority);
    get_forester_epoch_pda(&forester_pda.0, epoch)
}

/// Derives the forester epoch PDA from derivation key and epoch
pub fn get_forester_epoch_pda_from_derivation(derivation: &Pubkey, epoch: u64) -> (Pubkey, u8) {
    let forester_pda = get_forester_pda(derivation);
    get_forester_epoch_pda(&forester_pda.0, epoch)
}

/// Derives the protocol config PDA address
pub fn get_protocol_config_pda_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PROTOCOL_CONFIG_PDA_SEED], &REGISTRY_PROGRAM_ID)
}

/// Derives the epoch PDA address
pub fn get_epoch_pda_address(epoch: u64) -> Pubkey {
    Pubkey::find_program_address(&[&epoch.to_le_bytes()], &REGISTRY_PROGRAM_ID).0
}

/// Protocol configuration
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct ProtocolConfig {
    pub genesis_slot: u64,
    pub min_weight: u64,
    pub slot_length: u64,
    pub registration_phase_length: u64,
    pub active_phase_length: u64,
    pub report_work_phase_length: u64,
    pub network_fee: u64,
    pub cpi_context_size: u64,
    pub finalize_counter_limit: u64,
    pub place_holder: Pubkey,
    pub address_network_fee: u64,
}

/// Protocol config PDA account data
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct ProtocolConfigPda {
    pub authority: Pubkey,
    pub bump: u8,
    pub config: ProtocolConfig,
}
