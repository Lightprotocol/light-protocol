// Edge case: Enum variant with array
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for enums
#[repr(C)]
pub enum EnumWithArray {
    Empty,
    WithArray([u8; 32]),
}

fn main() {
    // Test Borsh compatibility
    let ref_enum = EnumWithArray::WithArray([42; 32]);
    let bytes = ref_enum.try_to_vec().unwrap();

    let (_enum_copy, remaining) = EnumWithArray::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation for enums
    assert!(remaining.is_empty());

    // Test ZeroCopyMut - mutable deserialization
    let mut bytes_mut = bytes.clone();
    let (_enum_mut, remaining) = EnumWithArray::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Test ZeroCopyNew - initialization with config
    let config = EnumWithArrayConfig::WithArray(());
    let byte_len = EnumWithArray::byte_len(&config).unwrap();
    
    let mut new_bytes = vec![0u8; byte_len];
    let (mut enum_new, remaining) = EnumWithArray::new_zero_copy(&mut new_bytes, config).unwrap();
    assert!(remaining.is_empty());
    
    // Initialize the array data
    match &mut enum_new {
        ZEnumWithArrayMut::WithArray(ref mut array_data) => {
            // Set array values
            for i in 0..32 {
                array_data[i] = 42;
            }
        }
        _ => panic!("Expected WithArray variant"),
    }
}
