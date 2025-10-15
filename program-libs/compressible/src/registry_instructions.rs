//! Client-side instruction builders for Light Registry operations
//!
//! This module provides instruction data structures and account meta builders
//! for creating compressible configs via the Light Registry program.

// Use Anchor's Pubkey when anchor feature is enabled, otherwise use solana-pubkey
#[cfg(feature = "anchor")]
use anchor_lang::prelude::Pubkey;
#[cfg(not(feature = "anchor"))]
use solana_pubkey::Pubkey;

use crate::{rent::RentConfig, AnchorDeserialize, AnchorSerialize};

/// Discriminator for CreateConfigCounter instruction
pub const CREATE_CONFIG_COUNTER_DISCRIMINATOR: [u8; 8] = [221, 9, 219, 187, 215, 138, 209, 87];

/// Discriminator for CreateCompressibleConfig instruction
pub const CREATE_COMPRESSIBLE_CONFIG_DISCRIMINATOR: [u8; 8] = [13, 182, 188, 82, 224, 82, 11, 174];

/// Instruction data for CreateConfigCounter
///
/// Creates the config counter PDA that tracks the number of compressible configs.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct CreateConfigCounter {}

impl CreateConfigCounter {
    /// Get the instruction discriminator
    pub const fn discriminator() -> [u8; 8] {
        CREATE_CONFIG_COUNTER_DISCRIMINATOR
    }

    /// Serialize instruction data including discriminator
    pub fn data(&self) -> Vec<u8> {
        let mut data = Self::discriminator().to_vec();
        data.extend_from_slice(&borsh::to_vec(self).unwrap());
        data
    }
}

/// Instruction data for CreateCompressibleConfig
///
/// Creates a new compressible config with the specified parameters.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct CreateCompressibleConfig {
    pub rent_config: RentConfig,
    pub update_authority: Pubkey,
    pub withdrawal_authority: Pubkey,
    pub active: bool,
}

impl CreateCompressibleConfig {
    /// Get the instruction discriminator
    pub const fn discriminator() -> [u8; 8] {
        CREATE_COMPRESSIBLE_CONFIG_DISCRIMINATOR
    }

    /// Serialize instruction data including discriminator
    pub fn data(&self) -> Vec<u8> {
        let mut data = Self::discriminator().to_vec();
        data.extend_from_slice(&AnchorSerialize::try_to_vec(self).unwrap());
        data
    }
}

/// Account metas for CreateCompressibleConfig instruction
#[derive(Debug, Clone)]
pub struct CreateCompressibleConfigAccounts {
    pub fee_payer: Pubkey,
    pub authority: Pubkey,
    pub protocol_config_pda: Pubkey,
    pub config_counter: Pubkey,
    pub compressible_config: Pubkey,
    pub system_program: Pubkey,
}

/// Utility functions for Light Registry PDAs
pub mod utils {
    use solana_pubkey::Pubkey;

    /// Light Registry program ID
    pub const LIGHT_REGISTRY_ID: Pubkey =
        solana_pubkey::pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX");

    /// Protocol config PDA seed
    pub const PROTOCOL_CONFIG_PDA_SEED: &[u8] = b"protocol_config";

    /// Get the protocol config PDA address
    pub fn get_protocol_config_pda_address() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[PROTOCOL_CONFIG_PDA_SEED], &LIGHT_REGISTRY_ID)
    }
}
