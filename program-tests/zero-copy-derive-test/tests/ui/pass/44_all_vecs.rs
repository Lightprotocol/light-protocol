// Edge case: All fields are Vecs

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct AllVecs {
    pub v1: Vec<u8>,
    pub v2: Vec<u16>,
    pub v3: Vec<u32>,
    pub v4: Vec<u64>,
    pub v5: Vec<bool>,
}

fn main() {
    let instance = AllVecs {
        v1: vec![1, 2, 3],
        v2: vec![100, 200],
        v3: vec![1000, 2000, 3000],
        v4: vec![10000, 20000],
        v5: vec![true, false, true],
    };

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();
    let deserialized = AllVecs::try_from_slice(&bytes).unwrap();

    // Test zero_copy_at
    let (zero_copy_instance, remaining) = AllVecs::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy_instance.v1.to_vec(), deserialized.v1);
    assert_eq!(zero_copy_instance.v2.to_vec(), deserialized.v2);
    assert!(remaining.is_empty());

    // Test zero_copy_at_mut
    let mut bytes_mut = bytes.clone();
    let (_zero_copy_mut, remaining) = AllVecs::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = AllVecsConfig {
        v1: 3,
        v2: 2,
        v3: 3,
        v4: 2,
        v5: 3,
    };
    let byte_len = AllVecs::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) = AllVecs::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.v1[0] = 1;
    struct_copy_mut.v1[1] = 2;
    struct_copy_mut.v1[2] = 3;
    struct_copy_mut.v2[0] = 100.into();
    struct_copy_mut.v2[1] = 200.into();
    struct_copy_mut.v3[0] = 1000.into();
    struct_copy_mut.v3[1] = 2000.into();
    struct_copy_mut.v3[2] = 3000.into();
    struct_copy_mut.v4[0] = 10000.into();
    struct_copy_mut.v4[1] = 20000.into();
    struct_copy_mut.v5[0] = 1; // true as u8
    struct_copy_mut.v5[1] = 0; // false as u8
    struct_copy_mut.v5[2] = 1; // true as u8
    assert_eq!(new_bytes, bytes);
}
