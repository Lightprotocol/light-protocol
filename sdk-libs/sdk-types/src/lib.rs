pub mod address;
pub mod constants;
pub mod cpi_accounts;
pub mod error;
pub mod instruction;

// Re-exports
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use constants::*;
pub use cpi_accounts::*;

/// Configuration struct containing program ID, CPI signer, and bump for Light Protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorDeserialize, AnchorSerialize)]
pub struct CpiSigner {
    pub program_id: [u8; 32],
    pub cpi_signer: [u8; 32],
    pub bump: u8,
}
