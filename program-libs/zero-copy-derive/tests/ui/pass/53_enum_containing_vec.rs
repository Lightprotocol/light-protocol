// Edge case: Enum containing Vec
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub enum EnumWithVec {
    Empty,
    Data(Vec<u8>),
    MoreData(Vec<u32>),
}

fn main() {
    let instance = EnumWithVec::Data(vec![1, 2, 3, 4, 5]);

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();

    // Test zero_copy_at
    let (zero_copy_instance, remaining) = EnumWithVec::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation for enums

    // Note: Enums do not support ZeroCopyEq
    println!("✓ EnumWithVec Borsh compatibility test passed");
}
