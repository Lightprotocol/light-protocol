use light_compressed_account::instruction_data::zero_copy_set::CompressedCpiContextTrait;
use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize, CMINT_ADDRESS_TREE};

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
    /// Placeholder to enable cmints in multiple address trees.
    /// Currently set to 0.
    pub read_only_address_trees: [u8; 4],
    pub address_tree_pubkey: [u8; 32],
}

impl Default for CpiContext {
    fn default() -> Self {
        Self {
            set_context: false,
            first_set_context: false,
            in_tree_index: 0,
            in_queue_index: 0,
            out_queue_index: 0,
            token_out_queue_index: 0,
            assigned_account_index: 0,
            read_only_address_trees: [0; 4],
            address_tree_pubkey: CMINT_ADDRESS_TREE,
        }
    }
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
