// Edge case: Fixed-size arrays
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct ArrayFields {
    pub small: [u8; 4],
    pub medium: [u32; 16],
    pub large: [u64; 256],
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = ArrayFields {
        small: [1, 2, 3, 4],
        medium: [10; 16],
        large: [100; 256],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = ArrayFields::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = ArrayFields::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}
