// Edge case: Single Vec field

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct SingleVec {
    pub data: Vec<u8>,
}

fn main() {
    let instance = SingleVec {
        data: vec![1, 2, 3, 4, 5],
    };

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();
    let deserialized = SingleVec::try_from_slice(&bytes).unwrap();

    // Test zero_copy_at
    let (zero_copy_instance, _remaining) = SingleVec::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy_instance.data.to_vec(), deserialized.data);
    assert!(_remaining.is_empty());

    // Test zero_copy_at_mut
    let mut bytes_mut = bytes.clone();
    let (_zero_copy_mut, _remaining) = SingleVec::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(_remaining.is_empty());

    // assert byte len
    let config = SingleVecConfig { data: 5 };
    let byte_len = SingleVec::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (struct_copy_mut, _remaining) = SingleVec::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.data[0] = 1;
    struct_copy_mut.data[1] = 2;
    struct_copy_mut.data[2] = 3;
    struct_copy_mut.data[3] = 4;
    struct_copy_mut.data[4] = 5;
    assert_eq!(new_bytes, bytes);

    // Note: Cannot use assert_eq! with entire structs due to Vec fields
    println!("✓ SingleVec Borsh compatibility test passed");
}
