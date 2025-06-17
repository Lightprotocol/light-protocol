use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(Debug, Copy, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CpiSigner {
    pub program_id: [u8; 32],
    pub bump: u8,
}
