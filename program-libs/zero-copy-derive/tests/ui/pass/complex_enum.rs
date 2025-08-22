#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq typically not used with enums
#[repr(C)]
pub enum ComplexEnum {
    UnitVariant,
    U64Field(u64), 
    BoolField(bool),
    U32Field(u32),
}

fn main() {
    // Test Borsh compatibility
    let ref_enum = ComplexEnum::U64Field(12345);
    let bytes = ref_enum.try_to_vec().unwrap();

    let (_enum_copy, remaining) = ComplexEnum::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());

    // Test ZeroCopyMut - mutable deserialization
    let mut bytes_mut = bytes.clone();
    let (mut enum_mut, remaining) = ComplexEnum::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
    
    // Can mutate data within existing variant (discriminant remains immutable)
    match &mut enum_mut {
        ZComplexEnumMut::U64Field(ref mut data) => {
            **data = 99999u64.into(); // Modify the u64 value
        }
        _ => panic!("Expected U64Field variant"),
    }

    // Test ZeroCopyNew - initialization with config
    let config = ComplexEnumConfig::U64Field(());
    let byte_len = ComplexEnum::byte_len(&config).unwrap();
    
    let mut new_bytes = vec![0u8; byte_len];
    let (mut enum_new, remaining) = ComplexEnum::new_zero_copy(&mut new_bytes, config).unwrap();
    assert!(remaining.is_empty());
    
    // Initialize the data field
    match &mut enum_new {
        ZComplexEnumMut::U64Field(ref mut data) => {
            **data = 12345u64.into();
        }
        _ => panic!("Expected U64Field variant"),
    }
}