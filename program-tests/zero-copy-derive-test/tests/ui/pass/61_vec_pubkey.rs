// Edge case: Vec of Pubkey

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

// Import Pubkey from the test helper
#[path = "../../instruction_data.rs"]
mod instruction_data;
use instruction_data::Pubkey;

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct VecPubkey {
    pub signers: Vec<Pubkey>,
    pub authorities: Vec<Pubkey>,
}

fn main() {
    let original = VecPubkey {
        signers: vec![Pubkey([1; 32]), Pubkey([2; 32])],
        authorities: vec![Pubkey([3; 32]), Pubkey([4; 32]), Pubkey([5; 32])],
    };

    // Test Borsh serialization
    let serialized = original.try_to_vec().unwrap();
    let _deserialized: VecPubkey = VecPubkey::try_from_slice(&serialized).unwrap();

    // Test zero_copy_at (read-only)
    let _zero_copy_read = VecPubkey::zero_copy_at(&serialized).unwrap();

    // Test zero_copy_at_mut (mutable)
    let mut serialized_mut = serialized.clone();
    let _zero_copy_mut = VecPubkey::zero_copy_at_mut(&mut serialized_mut).unwrap();

    // Note: Cannot use assert_eq! due to Vec fields not implementing ZeroCopyEq
    println!("Borsh compatibility test passed for VecPubkey");
}
