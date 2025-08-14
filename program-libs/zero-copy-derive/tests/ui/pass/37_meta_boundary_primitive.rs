// Edge case: Meta boundary with primitive types
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct MetaBoundaryPrimitive {
    pub a: u32,
    pub b: u64,
    pub c: bool,
    pub vec: Vec<u8>,  // Meta boundary here
    pub d: u32,
    pub e: u64,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = MetaBoundaryPrimitive {
        a: 10,
        b: 20,
        c: true,
        vec: vec![1, 2, 3],
        d: 30,
        e: 40,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = MetaBoundaryPrimitive::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = MetaBoundaryPrimitive::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}