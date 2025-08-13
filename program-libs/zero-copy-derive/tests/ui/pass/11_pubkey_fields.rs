// Edge case: Multiple Pubkey fields
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

// Import Pubkey from the test helper
#[path = "../../instruction_data.rs"]
mod instruction_data;
use instruction_data::Pubkey;

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct PubkeyFields {
    pub owner: Pubkey,
    pub authority: Pubkey,
    pub mint: Pubkey,
}

fn main() {}
