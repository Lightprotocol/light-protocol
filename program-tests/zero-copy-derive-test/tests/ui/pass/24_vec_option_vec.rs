// Edge case: Vec, Option, Vec pattern

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct VecOptionVec {
    pub first: Vec<u8>,
    pub middle: Option<u64>,
    pub last: Vec<u32>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = VecOptionVec {
        first: vec![1, 2, 3],
        middle: Some(42),
        last: vec![10, 20, 30],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = VecOptionVec::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = VecOptionVec::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Test ZeroCopyNew
    let config = VecOptionVecConfig {
        first: 3,
        middle: true,
        last: 3,
    };
    let byte_len = VecOptionVec::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        VecOptionVec::new_zero_copy(&mut new_bytes, config).unwrap();
    struct_copy_mut.first[0] = 1;
    struct_copy_mut.first[1] = 2;
    struct_copy_mut.first[2] = 3;
    if let Some(ref mut val) = struct_copy_mut.middle {
        **val = 42u64.into();
    }
    struct_copy_mut.last[0] = 10.into();
    struct_copy_mut.last[1] = 20.into();
    struct_copy_mut.last[2] = 30.into();
    assert_eq!(new_bytes, bytes);
}
