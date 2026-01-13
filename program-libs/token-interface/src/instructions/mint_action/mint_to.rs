use light_zero_copy::ZeroCopy;

use crate::{AnchorDeserialize, AnchorSerialize};

#[repr(C)]
#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct MintToAction {
    pub account_index: u8, // Index into remaining accounts for the recipient token account
    pub amount: u64,
}
