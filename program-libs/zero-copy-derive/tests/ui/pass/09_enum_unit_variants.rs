// Edge case: Enum with only unit variants
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub enum UnitEnum {
    First,
    Second,
    Third,
    Fourth,
    Fifth,
}

fn main() {
    // Test Borsh compatibility
    let ref_enum = UnitEnum::Third;
    let bytes = ref_enum.try_to_vec().unwrap();

    let (_enum_copy, remaining) = UnitEnum::zero_copy_at(&bytes).unwrap();
    // Note: ZeroCopyEq not supported for enums
    assert!(remaining.is_empty());

    // Test ZeroCopyMut - mutable deserialization
    let mut bytes_mut = bytes.clone();
    let (_enum_mut, remaining) = UnitEnum::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Test ZeroCopyNew - initialization with config
    let config = UnitEnumConfig::Third;
    let byte_len = UnitEnum::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    
    let mut new_bytes = vec![0u8; byte_len];
    let (_enum_new, remaining) = UnitEnum::new_zero_copy(&mut new_bytes, config).unwrap();
    assert!(remaining.is_empty());
    assert_eq!(new_bytes, bytes);
}
