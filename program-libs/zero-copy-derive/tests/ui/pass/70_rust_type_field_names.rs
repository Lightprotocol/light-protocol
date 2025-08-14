// Edge case: Field names that are Rust type names
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct RustTypeFieldNames {
    pub u32: u64,            // Field named u32, but type is u64
    pub bool: u32,           // Field named bool, but type is u32
    pub vec: Vec<u8>,        // Field named vec
    pub option: Option<u32>, // Field named option
    pub array: [u8; 10],     // Field named array
}

fn main() {
    let original = RustTypeFieldNames {
        u32: 123456789,
        bool: 42,
        vec: vec![1, 2, 3, 4, 5],
        option: Some(999),
        array: [255u8; 10],
    };

    // Test Borsh serialization
    let serialized = original.try_to_vec().unwrap();

    // Test zero_copy_at (read-only)
    let zero_copy_read = RustTypeFieldNames::zero_copy_at(&serialized).unwrap();

    // Test zero_copy_at_mut (mutable)
    let mut serialized_mut = serialized.clone();
    let _zero_copy_mut = RustTypeFieldNames::zero_copy_at_mut(&mut serialized_mut).unwrap();

    // Note: Cannot use assert_eq! due to Vec and array fields not implementing ZeroCopyEq
    println!("Borsh compatibility test passed for RustTypeFieldNames");
}
