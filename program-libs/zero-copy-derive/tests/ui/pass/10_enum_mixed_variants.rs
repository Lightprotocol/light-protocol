// Edge case: Enum with mixed variant types
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub enum MixedEnum {
    Empty,
    WithData(u32),
    Another,
}

fn main() {
    // Test Borsh compatibility
    let ref_enum = MixedEnum::WithData(42);
    let bytes = ref_enum.try_to_vec().unwrap();

    let (_enum_copy, remaining) = MixedEnum::zero_copy_at(&bytes).unwrap();
    // Note: ZeroCopyEq not supported for enums
    assert!(remaining.is_empty());

    // Test ZeroCopyMut - mutable deserialization
    let mut bytes_mut = bytes.clone();
    let (mut enum_mut, remaining) = MixedEnum::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Can mutate data within existing variant (discriminant remains immutable)
    match &mut enum_mut {
        ZMixedEnumMut::WithData(ref mut data) => {
            **data = 100u32.into(); // Modify the u32 value
        }
        _ => panic!("Expected WithData variant"),
    }

    // Test ZeroCopyNew - initialization with config
    let config = MixedEnumConfig::WithData(());
    let byte_len = MixedEnum::byte_len(&config).unwrap();

    let mut new_bytes = vec![0u8; byte_len];
    let (mut enum_new, remaining) = MixedEnum::new_zero_copy(&mut new_bytes, config).unwrap();
    assert!(remaining.is_empty());

    // Initialize the data field
    match &mut enum_new {
        ZMixedEnumMut::WithData(ref mut data) => {
            **data = 42u32.into();
        }
        _ => panic!("Expected WithData variant"),
    }
}
