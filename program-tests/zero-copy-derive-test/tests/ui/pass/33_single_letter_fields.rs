// Edge case: Single letter field names

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct SingleLetterFields {
    pub a: u8,
    pub b: u16,
    pub c: u32,
    pub d: u64,
    pub e: Vec<u8>,
    pub f: Option<u32>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = SingleLetterFields {
        a: 1,
        b: 2,
        c: 3,
        d: 4,
        e: vec![5, 6, 7],
        f: Some(8),
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = SingleLetterFields::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) =
        SingleLetterFields::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = SingleLetterFieldsConfig { e: 3, f: true };
    let byte_len = SingleLetterFields::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        SingleLetterFields::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.a = 1;
    struct_copy_mut.b = 2.into();
    struct_copy_mut.c = 3.into();
    struct_copy_mut.d = 4.into();
    struct_copy_mut.e[0] = 5;
    struct_copy_mut.e[1] = 6;
    struct_copy_mut.e[2] = 7;
    if let Some(ref mut val) = struct_copy_mut.f {
        **val = 8.into();
    }
    assert_eq!(new_bytes, bytes);
}
