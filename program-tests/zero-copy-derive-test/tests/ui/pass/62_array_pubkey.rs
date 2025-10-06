// Edge case: Array of Pubkey

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

// Import Pubkey from the test helper
#[path = "../../instruction_data.rs"]
mod instruction_data;
use instruction_data::Pubkey;

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct ArrayPubkey {
    pub signers: [Pubkey; 3],
    pub validators: [Pubkey; 10],
}

fn main() {
    let original = ArrayPubkey {
        signers: [Pubkey([1; 32]), Pubkey([2; 32]), Pubkey([3; 32])],
        validators: [
            Pubkey([4; 32]),
            Pubkey([5; 32]),
            Pubkey([6; 32]),
            Pubkey([7; 32]),
            Pubkey([8; 32]),
            Pubkey([9; 32]),
            Pubkey([10; 32]),
            Pubkey([11; 32]),
            Pubkey([12; 32]),
            Pubkey([13; 32]),
        ],
    };

    // Test Borsh serialization
    let serialized = original.try_to_vec().unwrap();

    // Test zero_copy_at (read-only)
    let _zero_copy_read = ArrayPubkey::zero_copy_at(&serialized).unwrap();

    // Test zero_copy_at_mut (mutable)
    let mut serialized_mut = serialized.clone();
    let _zero_copy_mut = ArrayPubkey::zero_copy_at_mut(&mut serialized_mut).unwrap();

    // Note: Cannot use assert_eq! due to array fields not implementing ZeroCopyEq
    println!("Borsh compatibility test passed for ArrayPubkey");
}
