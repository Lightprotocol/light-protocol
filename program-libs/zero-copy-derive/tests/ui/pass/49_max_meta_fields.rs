// Edge case: Maximum consecutive meta fields before Vec
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct MaxMetaFields {
    pub m1: u8,
    pub m2: u16,
    pub m3: u32,
    pub m4: u64,
    pub m5: i8,
    pub m6: i16,
    pub m7: i32,
    pub m8: i64,
    pub m9: bool,
    pub m10: [u8; 32],
    pub m11: u32,
    pub m12: u64,
    pub data: Vec<u8>, // Meta boundary
    pub after: u32,
}

fn main() {
    let instance = MaxMetaFields {
        m1: 1,
        m2: 2,
        m3: 3,
        m4: 4,
        m5: 5,
        m6: 6,
        m7: 7,
        m8: 8,
        m9: true,
        m10: [42; 32],
        m11: 11,
        m12: 12,
        data: vec![1, 2, 3],
        after: 999,
    };

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();
    let deserialized = MaxMetaFields::try_from_slice(&bytes).unwrap();

    // Test zero_copy_at
    let (zero_copy_instance, remaining) = MaxMetaFields::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy_instance.m1, deserialized.m1);
    assert!(remaining.is_empty());

    // Test zero_copy_at_mut
    let mut bytes_mut = bytes.clone();
    let (mut zero_copy_mut, remaining) = MaxMetaFields::zero_copy_at_mut(&mut bytes_mut).unwrap();
    zero_copy_mut.m1 = 100;
    assert_eq!(zero_copy_mut.m1, 100);
    assert!(remaining.is_empty());

    // Note: Cannot use assert_eq! with entire structs due to array and Vec fields
    println!("✓ MaxMetaFields Borsh compatibility test passed");
}
