// Edge case: Arrays of different sizes and types
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for array fields
#[repr(C)]
pub struct MixedArrays {
    pub tiny: [u8; 1],
    pub small: [u16; 8],
    pub medium: [u32; 32],
    pub large: [u64; 128],
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = MixedArrays {
        tiny: [1],
        small: [2; 8],
        medium: [3; 32],
        large: [4; 128],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = MixedArrays::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = MixedArrays::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}
