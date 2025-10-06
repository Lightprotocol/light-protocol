// Edge case: Field named "data" (potential naming conflict)

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct FieldNamedData {
    pub data: u32,
    pub bytes: Vec<u8>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = FieldNamedData {
        data: 42,
        bytes: vec![1, 2, 3],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = FieldNamedData::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = FieldNamedData::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = FieldNamedDataConfig { bytes: 3 };
    let byte_len = FieldNamedData::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        FieldNamedData::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.data = 42.into();
    struct_copy_mut.bytes[0] = 1;
    struct_copy_mut.bytes[1] = 2;
    struct_copy_mut.bytes[2] = 3;
    assert_eq!(new_bytes, bytes);
}
