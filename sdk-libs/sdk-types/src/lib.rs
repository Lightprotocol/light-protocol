pub mod address;
pub mod constants;
pub mod cpi_accounts;
#[cfg(feature = "small_ix")]
pub mod cpi_accounts_small;
pub mod error;
pub mod instruction;

// Re-exports
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use constants::*;
pub use cpi_accounts::*;
#[cfg(feature = "small_ix")]
pub use cpi_accounts_small::{
    CompressionCpiAccountIndexSmall, CpiAccountsSmall, PROGRAM_ACCOUNTS_LEN,
    SMALL_SYSTEM_ACCOUNTS_LEN,
};

/// Configuration struct containing program ID, CPI signer, and bump for Light Protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorDeserialize, AnchorSerialize)]
pub struct CpiSigner {
    pub program_id: [u8; 32],
    pub cpi_signer: [u8; 32],
    pub bump: u8,
}
