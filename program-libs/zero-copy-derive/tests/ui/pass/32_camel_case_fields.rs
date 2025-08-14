// Edge case: CamelCase field names
#![cfg(feature = "mut")]
#![allow(non_snake_case)]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct CamelCaseFields {
    pub MyField: u32,
    pub AnotherField: Vec<u8>,
    pub YetAnotherField: Option<u64>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = CamelCaseFields {
        MyField: 42,
        AnotherField: vec![1, 2, 3],
        YetAnotherField: Some(100),
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = CamelCaseFields::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = CamelCaseFields::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}
