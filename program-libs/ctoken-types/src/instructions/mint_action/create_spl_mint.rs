use light_zero_copy::ZeroCopy;

use crate::{AnchorDeserialize, AnchorSerialize};

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct CreateSplMintAction {
    pub mint_bump: u8,
}
