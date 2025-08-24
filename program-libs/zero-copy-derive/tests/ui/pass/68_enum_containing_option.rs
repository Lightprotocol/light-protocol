// Edge case: Enum containing Option
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub enum EnumWithOption {
    Empty,
    MaybeData(Option<u32>),
    MaybeVec(Option<Vec<u8>>),
}

fn main() {
    let original = EnumWithOption::MaybeData(Some(42));

    // Test Borsh serialization
    let serialized = original.try_to_vec().unwrap();

    // Test zero_copy_at (read-only)
    let (_zero_copy_read, remaining) = EnumWithOption::zero_copy_at(&serialized).unwrap();
    assert!(remaining.is_empty());

    // Test ZeroCopyMut - mutable deserialization
    let mut bytes_mut = serialized.clone();
    let (_enum_mut, remaining) = EnumWithOption::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Test ZeroCopyNew - initialization with config
    let config = EnumWithOptionConfig::MaybeData((true, ()));
    let byte_len = EnumWithOption::byte_len(&config).unwrap();

    let mut new_bytes = vec![0u8; byte_len];
    let (_enum_new, remaining) = EnumWithOption::new_zero_copy(&mut new_bytes, config).unwrap();
    assert!(remaining.is_empty());

    // Note: Cannot use assert_eq! as enums don't implement ZeroCopyEq
    println!("Borsh compatibility test passed for EnumWithOption");
}
