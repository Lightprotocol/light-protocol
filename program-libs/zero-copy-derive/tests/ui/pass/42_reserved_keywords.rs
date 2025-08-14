// Edge case: Fields with names close to reserved keywords
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct ReservedKeywords {
    pub type_: u32,
    pub ref_: u64,
    pub mut_: bool,
    pub fn_: Vec<u8>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = ReservedKeywords {
        type_: 10,
        ref_: 20,
        mut_: true,
        fn_: vec![1, 2, 3],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = ReservedKeywords::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = ReservedKeywords::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}
