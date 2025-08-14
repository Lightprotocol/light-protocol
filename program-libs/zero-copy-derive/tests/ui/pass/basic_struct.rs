#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, ZeroCopyEq, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct BasicStruct {
    pub field1: u32,
    pub field2: u64,
    pub field3: bool,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = BasicStruct {
        field1: 42,
        field2: 1337,
        field3: true,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (struct_copy, remaining) = BasicStruct::zero_copy_at(&bytes).unwrap();
    assert_eq!(struct_copy, ref_struct);
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = BasicStruct::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}
