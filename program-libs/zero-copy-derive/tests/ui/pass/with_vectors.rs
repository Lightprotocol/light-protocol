#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct WithVectors {
    pub data: Vec<u8>,
    pub numbers: Vec<u32>,
    pub flags: Vec<bool>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = WithVectors {
        data: vec![1, 2, 3, 4],
        numbers: vec![10, 20, 30],
        flags: vec![true, false, true],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = WithVectors::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = WithVectors::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}