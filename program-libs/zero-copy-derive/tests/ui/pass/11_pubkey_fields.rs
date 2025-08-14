// Edge case: Multiple Pubkey fields
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq, ZeroCopyMut};

// Import Pubkey from the test helper
#[path = "../../instruction_data.rs"]
mod instruction_data;
use instruction_data::Pubkey;

#[derive(Debug, ZeroCopy, ZeroCopyEq, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct PubkeyFields {
    pub owner: Pubkey,
    pub authority: Pubkey,
    pub mint: Pubkey,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = PubkeyFields {
        owner: Pubkey([1; 32]),
        authority: Pubkey([2; 32]),
        mint: Pubkey([3; 32]),
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (struct_copy, remaining) = PubkeyFields::zero_copy_at(&bytes).unwrap();
    assert_eq!(struct_copy, ref_struct);
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = PubkeyFields::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}
