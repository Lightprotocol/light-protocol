// Edge case: Fields with underscores

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct UnderscoreFields {
    pub _reserved: u32,
    pub __internal: u64,
    pub normal_field: Vec<u8>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = UnderscoreFields {
        _reserved: 100,
        __internal: 200,
        normal_field: vec![1, 2, 3],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = UnderscoreFields::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = UnderscoreFields::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = UnderscoreFieldsConfig { normal_field: 3 };
    let byte_len = UnderscoreFields::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        UnderscoreFields::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut._reserved = 100.into();
    struct_copy_mut.__internal = 200.into();
    struct_copy_mut.normal_field[0] = 1;
    struct_copy_mut.normal_field[1] = 2;
    struct_copy_mut.normal_field[2] = 3;
    assert_eq!(new_bytes, bytes);
}
