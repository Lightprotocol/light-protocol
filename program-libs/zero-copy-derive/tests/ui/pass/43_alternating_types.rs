// Edge case: Alternating primitive and dynamic types
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct AlternatingTypes {
    pub p1: u32,
    pub v1: Vec<u8>,
    pub p2: u64,
    pub v2: Vec<u32>,
    pub p3: bool,
    pub o1: Option<u64>,
}

fn main() {
    let instance = AlternatingTypes {
        p1: 42,
        v1: vec![1, 2, 3],
        p2: 12345,
        v2: vec![100, 200],
        p3: true,
        o1: Some(999),
    };

    // Test Borsh compatibility
    let bytes = instance.try_to_vec().unwrap();

    let (_struct_copy, remaining) = AlternatingTypes::zero_copy_at(&bytes).unwrap();
    // Note: Can't compare entire structs due to Vec fields, but can check primitive
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = AlternatingTypes::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}
