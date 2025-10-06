// Edge case: Deeply nested structs (3+ levels)

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct Level3 {
    pub value: u32,
}

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct Level2 {
    pub inner: Level3,
    pub data: u64,
}

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct Level1 {
    pub inner: Level2,
    pub extra: Vec<u8>,
}

fn main() {
    let instance = Level1 {
        inner: Level2 {
            inner: Level3 { value: 42 },
            data: 12345,
        },
        extra: vec![1, 2, 3, 4],
    };

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();

    // Test zero_copy_at
    let (_zero_copy_instance, remaining) = Level1::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());

    // Test zero_copy_at_mut
    let mut bytes_mut = bytes.clone();
    let (_zero_copy_mut, remaining) = Level1::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = Level1Config {
        inner: Level2Config { inner: () },
        extra: 4,
    };
    let byte_len = Level1::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) = Level1::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.inner.inner.value = 42.into();
    *struct_copy_mut.inner.data = 12345.into();
    struct_copy_mut.extra[0] = 1;
    struct_copy_mut.extra[1] = 2;
    struct_copy_mut.extra[2] = 3;
    struct_copy_mut.extra[3] = 4;
    assert_eq!(new_bytes, bytes);

    // Note: Cannot use assert_eq! with entire structs due to Vec fields
    println!("✓ Level1 Borsh compatibility test passed");
}
