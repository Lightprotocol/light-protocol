// Edge case: All three derives together
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct AllDerives {
    pub value: u32,
    pub data: Vec<u8>,
    pub optional: Option<u64>,
}

fn main() {
    let instance = AllDerives {
        value: 42,
        data: vec![1, 2, 3, 4],
        optional: Some(999),
    };

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();
    let deserialized = AllDerives::try_from_slice(&bytes).unwrap();

    // Test zero_copy_at
    let (zero_copy_instance, remaining) = AllDerives::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy_instance.value, deserialized.value);
    assert!(remaining.is_empty());

    // Test zero_copy_at_mut
    let mut bytes_mut = bytes.clone();
    let (mut zero_copy_mut, remaining) = AllDerives::zero_copy_at_mut(&mut bytes_mut).unwrap();
    zero_copy_mut.value = 777.into();
    assert_eq!(zero_copy_mut.value, 777);
    assert!(remaining.is_empty());

    // Note: Cannot use assert_eq! with entire structs due to Vec fields
    // ZeroCopyEq was removed because this struct has Vec fields
    println!("✓ AllDerives Borsh compatibility test passed");
}
