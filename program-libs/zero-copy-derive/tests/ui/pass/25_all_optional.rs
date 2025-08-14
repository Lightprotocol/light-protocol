// Edge case: All fields are optional
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct AllOptional {
    pub maybe_a: Option<u32>,
    pub maybe_b: Option<u64>,
    pub maybe_c: Option<Vec<u8>>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = AllOptional {
        maybe_a: Some(42),
        maybe_b: None,
        maybe_c: Some(vec![1, 2, 3]),
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = AllOptional::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = AllOptional::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}