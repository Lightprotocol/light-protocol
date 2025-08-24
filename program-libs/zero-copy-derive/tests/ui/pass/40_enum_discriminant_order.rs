// Edge case: Many enum variants (testing discriminant handling)
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for enums
#[repr(C)]
pub enum ManyVariants {
    V0,
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
    V9,
}

fn main() {
    // Test Borsh compatibility
    let ref_enum = ManyVariants::V5;
    let bytes = ref_enum.try_to_vec().unwrap();

    let (_enum_copy, remaining) = ManyVariants::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation for enums
    assert!(remaining.is_empty());

    // Test ZeroCopyMut - mutable deserialization
    let mut bytes_mut = bytes.clone();
    let (_enum_mut, remaining) = ManyVariants::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Test ZeroCopyNew - initialization with config
    let config = ManyVariantsConfig::V5;
    let byte_len = ManyVariants::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    
    let mut new_bytes = vec![0u8; byte_len];
    let (_enum_new, remaining) = ManyVariants::new_zero_copy(&mut new_bytes, config).unwrap();
    assert!(remaining.is_empty());
    assert_eq!(new_bytes, bytes);
}
