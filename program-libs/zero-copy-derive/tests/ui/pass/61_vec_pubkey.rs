// Edge case: Vec of Pubkey
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

// Import Pubkey from the test helper
#[path = "../../instruction_data.rs"]
mod instruction_data;
use instruction_data::Pubkey;

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct VecPubkey {
    pub signers: Vec<Pubkey>,
    pub authorities: Vec<Pubkey>,
}

fn main() {}