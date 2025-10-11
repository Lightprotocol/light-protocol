// Edge case: Fields with numeric suffixes

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct NumericSuffixFields {
    pub field1: u32,
    pub field2: u32,
    pub field3: u32,
    pub data1: Vec<u8>,
    pub data2: Vec<u16>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = NumericSuffixFields {
        field1: 10,
        field2: 20,
        field3: 30,
        data1: vec![1, 2, 3],
        data2: vec![100, 200],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = NumericSuffixFields::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) =
        NumericSuffixFields::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = NumericSuffixFieldsConfig { data1: 3, data2: 2 };
    let byte_len = NumericSuffixFields::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        NumericSuffixFields::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.field1 = 10.into();
    struct_copy_mut.field2 = 20.into();
    struct_copy_mut.field3 = 30.into();
    struct_copy_mut.data1[0] = 1;
    struct_copy_mut.data1[1] = 2;
    struct_copy_mut.data1[2] = 3;
    struct_copy_mut.data2[0] = 100.into();
    struct_copy_mut.data2[1] = 200.into();
    assert_eq!(new_bytes, bytes);
}
