// Edge case: Enum with single variant
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq typically not used with enums
#[repr(C)]
pub enum SingleVariant {
    Only,
}

fn main() {
    // Test Borsh compatibility
    let ref_enum = SingleVariant::Only;
    let bytes = ref_enum.try_to_vec().unwrap();

    let (_enum_copy, remaining) = SingleVariant::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());

    // Test ZeroCopyMut - mutable deserialization
    let mut bytes_mut = bytes.clone();
    let (_enum_mut, remaining) = SingleVariant::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Test ZeroCopyNew - initialization with config
    let config = SingleVariantConfig::Only;
    let byte_len = SingleVariant::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    
    let mut new_bytes = vec![0u8; byte_len];
    let (_enum_new, remaining) = SingleVariant::new_zero_copy(&mut new_bytes, config).unwrap();
    assert!(remaining.is_empty());
    assert_eq!(new_bytes, bytes);
}
