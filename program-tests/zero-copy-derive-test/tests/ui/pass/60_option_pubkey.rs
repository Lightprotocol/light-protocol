// Edge case: Option of Pubkey

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

// Import Pubkey from the test helper
#[path = "../../instruction_data.rs"]
mod instruction_data;
use instruction_data::Pubkey;

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct OptionPubkey {
    pub maybe_owner: Option<Pubkey>,
    pub maybe_authority: Option<Pubkey>,
}

fn main() {
    let instance = OptionPubkey {
        maybe_owner: Some(Pubkey([1; 32])),
        maybe_authority: None,
    };

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();

    // Test zero_copy_at
    let (_zero_copy_instance, remaining) = OptionPubkey::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());

    // Test zero_copy_at_mut
    let mut bytes_mut = bytes.clone();
    let (_zero_copy_mut, remaining) = OptionPubkey::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Note: Cannot use assert_eq! with entire structs due to Pubkey fields
    println!("✓ OptionPubkey Borsh compatibility test passed");
}
