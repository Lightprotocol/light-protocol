// Edge case: Zero-sized array
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, ZeroCopyEq, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct ZeroSizedArray {
    pub empty: [u8; 0],
    pub value: u32,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = ZeroSizedArray {
        empty: [],
        value: 42,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (struct_copy, remaining) = ZeroSizedArray::zero_copy_at(&bytes).unwrap();
    assert_eq!(struct_copy, ref_struct);
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = ZeroSizedArray::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}
