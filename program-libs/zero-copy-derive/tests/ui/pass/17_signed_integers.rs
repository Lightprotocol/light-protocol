// Edge case: All signed integer types
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut, ZeroCopyEq};
use borsh::{BorshSerialize, BorshDeserialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, ZeroCopyEq, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct SignedIntegers {
    pub tiny: i8,
    pub small: i16,
    pub medium: i32,
    pub large: i64,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = SignedIntegers {
        tiny: -1,
        small: -100,
        medium: -1000,
        large: -10000,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (struct_copy, remaining) = SignedIntegers::zero_copy_at(&bytes).unwrap();
    assert_eq!(struct_copy, ref_struct);
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = SignedIntegers::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}
