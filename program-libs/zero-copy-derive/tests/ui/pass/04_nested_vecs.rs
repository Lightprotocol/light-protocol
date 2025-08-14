// Edge case: Multiple Vec fields
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct NestedVecs {
    pub bytes: Vec<u8>,
    pub nums: Vec<u32>,
    pub more: Vec<u64>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = NestedVecs {
        bytes: vec![1, 2, 3],
        nums: vec![10, 20],
        more: vec![100, 200, 300, 400],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = NestedVecs::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = NestedVecs::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}
