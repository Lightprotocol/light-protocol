// Edge case: CamelCase field names

#![allow(non_snake_case)]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct CamelCaseFields {
    pub MyField: u32,
    pub AnotherField: Vec<u8>,
    pub YetAnotherField: Option<u64>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = CamelCaseFields {
        MyField: 42,
        AnotherField: vec![1, 2, 3],
        YetAnotherField: Some(100),
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = CamelCaseFields::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = CamelCaseFields::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = CamelCaseFieldsConfig {
        AnotherField: 3,
        YetAnotherField: true,
    };
    let byte_len = CamelCaseFields::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        CamelCaseFields::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.MyField = 42.into();
    struct_copy_mut.AnotherField[0] = 1;
    struct_copy_mut.AnotherField[1] = 2;
    struct_copy_mut.AnotherField[2] = 3;
    if let Some(ref mut val) = struct_copy_mut.YetAnotherField {
        **val = 100.into();
    }
    assert_eq!(new_bytes, bytes);
}
