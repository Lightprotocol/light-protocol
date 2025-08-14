// Edge case: Single Vec field
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct SingleVec {
    pub data: Vec<u8>,
}

fn main() {
    let instance = SingleVec {
        data: vec![1, 2, 3, 4, 5],
    };

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();
    let deserialized = SingleVec::try_from_slice(&bytes).unwrap();

    // Test zero_copy_at
    let (zero_copy_instance, remaining) = SingleVec::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy_instance.data.to_vec(), deserialized.data);
    assert!(remaining.is_empty());

    // Test zero_copy_at_mut
    let mut bytes_mut = bytes.clone();
    let (_zero_copy_mut, remaining) = SingleVec::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Note: Cannot use assert_eq! with entire structs due to Vec fields
    println!("✓ SingleVec Borsh compatibility test passed");
}
