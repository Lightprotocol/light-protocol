#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};

#[derive(Debug, Copy, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CpiSigner {
    pub program_id: [u8; 32],
    pub bump: u8,
}