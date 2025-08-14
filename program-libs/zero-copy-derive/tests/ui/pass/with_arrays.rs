#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, ZeroCopyEq, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct WithArrays {
    pub buffer: [u8; 32],
    pub data: [u8; 16],
    pub small_array: [u8; 4],
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = WithArrays {
        buffer: [1; 32],
        data: [2; 16],
        small_array: [3, 4, 5, 6],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (struct_copy, remaining) = WithArrays::zero_copy_at(&bytes).unwrap();
    assert_eq!(struct_copy, ref_struct);
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = WithArrays::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}
