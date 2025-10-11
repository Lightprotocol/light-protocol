// Edge case: Primitive fields after Vec (no meta optimization)

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct PrimitiveAfterVec {
    pub data: Vec<u8>,
    pub count: u32,
    pub flag: bool,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = PrimitiveAfterVec {
        data: vec![1, 2, 3],
        count: 100,
        flag: true,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = PrimitiveAfterVec::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) =
        PrimitiveAfterVec::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Test ZeroCopyNew
    let config = PrimitiveAfterVecConfig { data: 3 };
    let byte_len = PrimitiveAfterVec::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, remaining) =
        PrimitiveAfterVec::new_zero_copy(&mut new_bytes, config).unwrap();
    assert!(remaining.is_empty());
    struct_copy_mut.data[0] = 1;
    struct_copy_mut.data[1] = 2;
    struct_copy_mut.data[2] = 3;
    *struct_copy_mut.count = 100.into();
    *struct_copy_mut.flag = 1; // true as u8
    assert_eq!(new_bytes, bytes);
}
