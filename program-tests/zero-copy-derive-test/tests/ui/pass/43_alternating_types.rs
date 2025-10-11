// Edge case: Alternating primitive and dynamic types

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct AlternatingTypes {
    pub p1: u32,
    pub v1: Vec<u8>,
    pub p2: u64,
    pub v2: Vec<u32>,
    pub p3: bool,
    pub o1: Option<u64>,
}

fn main() {
    let instance = AlternatingTypes {
        p1: 42,
        v1: vec![1, 2, 3],
        p2: 12345,
        v2: vec![100, 200],
        p3: true,
        o1: Some(999),
    };

    // Test Borsh compatibility
    let bytes = instance.try_to_vec().unwrap();

    let (_struct_copy, remaining) = AlternatingTypes::zero_copy_at(&bytes).unwrap();
    // Note: Can't compare entire structs due to Vec fields, but can check primitive
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = AlternatingTypes::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = AlternatingTypesConfig {
        v1: 3,
        v2: 2,
        o1: true,
    };
    let byte_len = AlternatingTypes::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        AlternatingTypes::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.p1 = 42.into();
    struct_copy_mut.v1[0] = 1;
    struct_copy_mut.v1[1] = 2;
    struct_copy_mut.v1[2] = 3;
    *struct_copy_mut.p2 = 12345.into();
    struct_copy_mut.v2[0] = 100.into();
    struct_copy_mut.v2[1] = 200.into();
    *struct_copy_mut.p3 = 1; // true as u8
    if let Some(ref mut val) = struct_copy_mut.o1 {
        **val = 999.into();
    }
    assert_eq!(new_bytes, bytes);
}
