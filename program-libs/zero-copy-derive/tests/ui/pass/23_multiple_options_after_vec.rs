// Edge case: Multiple Options after Vec
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct OptionsAfterVec {
    pub data: Vec<u8>,
    pub opt1: Option<u32>,
    pub opt2: Option<u64>,
    pub opt3: Option<u16>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = OptionsAfterVec {
        data: vec![1, 2, 3, 4],
        opt1: Some(42),
        opt2: None,
        opt3: Some(100),
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = OptionsAfterVec::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = OptionsAfterVec::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}
