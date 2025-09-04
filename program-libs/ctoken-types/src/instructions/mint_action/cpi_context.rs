use light_compressed_account::instruction_data::zero_copy_set::CompressedCpiContextTrait;
use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

#[repr(C)]
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopy, ZeroCopyMut, PartialEq)]
pub struct CpiContext {
    pub set_context: bool,
    pub first_set_context: bool,
    // Used as address tree index if create mint
    pub in_tree_index: u8,
    pub in_queue_index: u8,
    pub out_queue_index: u8,
    pub token_out_queue_index: u8,
    // Index of the compressed account that should receive the new address (0 = mint, 1+ = token accounts)
    pub assigned_account_index: u8,
}

impl CompressedCpiContextTrait for ZCpiContext<'_> {
    fn first_set_context(&self) -> u8 {
        if self.first_set_context == 0 {
            0
        } else {
            1
        }
    }

    fn set_context(&self) -> u8 {
        if self.set_context == 0 {
            0
        } else {
            1
        }
    }
}
