use light_compressed_account::Pubkey;
use light_zero_copy::ZeroCopy;

use crate::{AnchorDeserialize, AnchorSerialize};

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct Recipient {
    pub recipient: Pubkey,
    pub amount: u64,
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct MintToCompressedAction {
    pub token_account_version: u8,
    pub recipients: Vec<Recipient>,
}
