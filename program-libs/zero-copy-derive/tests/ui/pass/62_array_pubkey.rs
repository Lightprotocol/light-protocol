// Edge case: Array of Pubkey
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

// Import Pubkey from the test helper
#[path = "../../instruction_data.rs"]
mod instruction_data;
use instruction_data::Pubkey;

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct ArrayPubkey {
    pub signers: [Pubkey; 3],
    pub validators: [Pubkey; 10],
}

fn main() {}