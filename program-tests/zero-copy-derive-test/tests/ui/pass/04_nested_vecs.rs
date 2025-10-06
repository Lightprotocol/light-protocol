// Edge case: Multiple Vec fields

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct NestedVecs {
    pub bytes: Vec<u8>,
    pub nums: Vec<u32>,
    pub more: Vec<u64>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = NestedVecs {
        bytes: vec![1, 2, 3],
        nums: vec![10, 20],
        more: vec![100, 200, 300, 400],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = NestedVecs::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = NestedVecs::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = NestedVecsConfig {
        bytes: 3,
        nums: 2,
        more: 4,
    };
    let byte_len = NestedVecs::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        NestedVecs::new_zero_copy(&mut new_bytes, config).unwrap();
    // populate vec fields
    struct_copy_mut.bytes[0] = 1;
    struct_copy_mut.bytes[1] = 2;
    struct_copy_mut.bytes[2] = 3;
    struct_copy_mut.nums[0] = 10.into();
    struct_copy_mut.nums[1] = 20.into();
    struct_copy_mut.more[0] = 100.into();
    struct_copy_mut.more[1] = 200.into();
    struct_copy_mut.more[2] = 300.into();
    struct_copy_mut.more[3] = 400.into();
    assert_eq!(new_bytes, bytes);
}
