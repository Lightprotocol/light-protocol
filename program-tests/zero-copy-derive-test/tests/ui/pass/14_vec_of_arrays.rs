// Edge case: Vec containing arrays

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec fields
#[repr(C)]
pub struct VecOfArrays {
    pub data: Vec<[u8; 32]>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = VecOfArrays {
        data: vec![[1; 32], [2; 32]],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = VecOfArrays::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec fields
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = VecOfArrays::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = VecOfArraysConfig { data: 2 };
    let byte_len = VecOfArrays::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        VecOfArrays::new_zero_copy(&mut new_bytes, config).unwrap();
    // set array values in the vec
    for i in 0..32 {
        struct_copy_mut.data[0][i] = 1;
        struct_copy_mut.data[1][i] = 2;
    }
    assert_eq!(new_bytes, bytes);
}
