// Edge case: Struct with field that could conflict with lifetime
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct LifetimeName {
    pub a: u32,
    pub lifetime: u64,
    pub static_field: Vec<u8>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = LifetimeName {
        a: 10,
        lifetime: 42,
        static_field: vec![1, 2, 3],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = LifetimeName::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = LifetimeName::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}