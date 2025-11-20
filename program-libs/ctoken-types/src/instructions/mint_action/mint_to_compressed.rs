use light_compressed_account::{pubkey::AsPubkey, Pubkey};
use light_zero_copy::ZeroCopy;

use crate::{AnchorDeserialize, AnchorSerialize};

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct Recipient {
    pub recipient: Pubkey,
    pub amount: u64,
}

impl Recipient {
    pub fn new(recipient: impl AsPubkey, amount: u64) -> Self {
        Self {
            recipient: recipient.to_light_pubkey(),
            amount,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct MintToCompressedAction {
    pub token_account_version: u8,
    pub recipients: Vec<Recipient>,
}

impl MintToCompressedAction {
    pub fn new(recipients: Vec<Recipient>) -> Self {
        Self {
            token_account_version: 3,
            recipients,
        }
    }
}
