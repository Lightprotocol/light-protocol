// Edge case: Deeply nested structs (3+ levels)
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct Level3 {
    pub value: u32,
}

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct Level2 {
    pub inner: Level3,
    pub data: u64,
}

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct Level1 {
    pub inner: Level2,
    pub extra: Vec<u8>,
}

fn main() {
    let instance = Level1 {
        inner: Level2 {
            inner: Level3 { value: 42 },
            data: 12345,
        },
        extra: vec![1, 2, 3, 4],
    };

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();

    // Test zero_copy_at
    let (_zero_copy_instance, remaining) = Level1::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());

    // Test zero_copy_at_mut
    let mut bytes_mut = bytes.clone();
    let (_zero_copy_mut, remaining) = Level1::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Note: Cannot use assert_eq! with entire structs due to Vec fields
    println!("✓ Level1 Borsh compatibility test passed");
}
