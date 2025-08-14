// Edge case: Fields with numeric suffixes
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct NumericSuffixFields {
    pub field1: u32,
    pub field2: u32,
    pub field3: u32,
    pub data1: Vec<u8>,
    pub data2: Vec<u16>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = NumericSuffixFields {
        field1: 10,
        field2: 20,
        field3: 30,
        data1: vec![1, 2, 3],
        data2: vec![100, 200],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = NumericSuffixFields::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = NumericSuffixFields::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}
