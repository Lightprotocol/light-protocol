// Edge case: All fields are Vecs
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct AllVecs {
    pub v1: Vec<u8>,
    pub v2: Vec<u16>,
    pub v3: Vec<u32>,
    pub v4: Vec<u64>,
    pub v5: Vec<bool>,
}

fn main() {
    let instance = AllVecs {
        v1: vec![1, 2, 3],
        v2: vec![100, 200],
        v3: vec![1000, 2000, 3000],
        v4: vec![10000, 20000],
        v5: vec![true, false, true],
    };

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();
    let deserialized = AllVecs::try_from_slice(&bytes).unwrap();

    // Test zero_copy_at
    let (zero_copy_instance, remaining) = AllVecs::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy_instance.v1.to_vec(), deserialized.v1);
    assert_eq!(zero_copy_instance.v2.to_vec(), deserialized.v2);
    assert!(remaining.is_empty());

    // Test zero_copy_at_mut
    let mut bytes_mut = bytes.clone();
    let (_zero_copy_mut, remaining) = AllVecs::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}
