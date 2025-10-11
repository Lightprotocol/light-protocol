// Edge case: Struct containing other zero-copy structs

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct Inner {
    pub value: u32,
}

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct Outer {
    pub inner: Inner,
    pub data: Vec<u8>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = Outer {
        inner: Inner { value: 42 },
        data: vec![1, 2, 3, 4],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = Outer::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = Outer::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Test ZeroCopyNew
    let config = OuterConfig { inner: (), data: 4 };
    let byte_len = Outer::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, remaining) = Outer::new_zero_copy(&mut new_bytes, config).unwrap();
    assert!(remaining.is_empty());
    struct_copy_mut.inner.value = 42.into();
    struct_copy_mut.data[0] = 1;
    struct_copy_mut.data[1] = 2;
    struct_copy_mut.data[2] = 3;
    struct_copy_mut.data[3] = 4;
    assert_eq!(new_bytes, bytes);
}
