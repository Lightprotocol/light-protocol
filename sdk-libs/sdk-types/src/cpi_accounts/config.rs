#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};

use crate::CpiSigner;

#[derive(Debug, Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct CpiAccountsConfig {
    pub cpi_context: bool,
    pub sol_compression_recipient: bool,
    pub sol_pool_pda: bool,
    pub cpi_signer: CpiSigner,
}

impl CpiAccountsConfig {
    pub const fn new(cpi_signer: CpiSigner) -> Self {
        Self {
            cpi_context: false,
            sol_compression_recipient: false,
            sol_pool_pda: false,
            cpi_signer,
        }
    }

    pub const fn new_with_cpi_context(cpi_signer: CpiSigner) -> Self {
        Self {
            cpi_context: true,
            sol_compression_recipient: false,
            sol_pool_pda: false,
            cpi_signer,
        }
    }

    pub fn cpi_signer(&self) -> [u8; 32] {
        self.cpi_signer.cpi_signer
    }

    pub fn bump(&self) -> u8 {
        self.cpi_signer.bump
    }
}
