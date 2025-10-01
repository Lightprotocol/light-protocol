pub mod address;
pub mod constants;
pub mod cpi_accounts;
#[cfg(feature = "v2")]
pub mod cpi_accounts_small;
#[cfg(feature = "v2_ix")]
pub mod cpi_accounts_v2;
pub mod cpi_context_write;
pub mod error;
pub mod instruction;

// Re-exports
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use constants::*;
pub use cpi_accounts::*;
#[cfg(feature = "v2")]
pub use cpi_accounts_small::{
    CompressionCpiAccountIndexSmall, CpiAccountsSmall,
    PROGRAM_ACCOUNTS_LEN as SMALL_PROGRAM_ACCOUNTS_LEN, SMALL_SYSTEM_ACCOUNTS_LEN,
};
#[cfg(feature = "v2_ix")]
pub use cpi_accounts_v2::{
    CompressionCpiAccountIndexV2, CpiAccountsV2, PROGRAM_ACCOUNTS_LEN as V2_PROGRAM_ACCOUNTS_LEN,
    V2_SYSTEM_ACCOUNTS_LEN,
};

/// Configuration struct containing program ID, CPI signer, and bump for Light Protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, AnchorDeserialize, AnchorSerialize)]
pub struct CpiSigner {
    pub program_id: [u8; 32],
    pub cpi_signer: [u8; 32],
    pub bump: u8,
}
