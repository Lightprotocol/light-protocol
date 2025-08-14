// Edge case: Vec, Option, Vec pattern
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct VecOptionVec {
    pub first: Vec<u8>,
    pub middle: Option<u64>,
    pub last: Vec<u32>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = VecOptionVec {
        first: vec![1, 2, 3],
        middle: Some(42),
        last: vec![10, 20, 30],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = VecOptionVec::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = VecOptionVec::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}