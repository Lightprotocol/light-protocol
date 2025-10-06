// Edge case: All three derives together

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct AllDerives {
    pub value: u32,
    pub data: Vec<u8>,
    pub optional: Option<u64>,
}

fn main() {
    let instance = AllDerives {
        value: 42,
        data: vec![1, 2, 3, 4],
        optional: Some(999),
    };

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();
    let deserialized = AllDerives::try_from_slice(&bytes).unwrap();

    // Test zero_copy_at
    let (zero_copy_instance, remaining) = AllDerives::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy_instance.value, deserialized.value);
    assert!(remaining.is_empty());

    // Test zero_copy_at_mut
    let mut bytes_mut = bytes.clone();
    let (mut zero_copy_mut, remaining) = AllDerives::zero_copy_at_mut(&mut bytes_mut).unwrap();
    zero_copy_mut.value = 777.into();
    assert_eq!(zero_copy_mut.value, 777);
    assert!(remaining.is_empty());

    // assert byte len
    let config = AllDerivesConfig {
        data: 4,
        optional: true,
    };
    let byte_len = AllDerives::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        AllDerives::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.value = 42.into();
    struct_copy_mut.data[0] = 1;
    struct_copy_mut.data[1] = 2;
    struct_copy_mut.data[2] = 3;
    struct_copy_mut.data[3] = 4;
    if let Some(ref mut val) = struct_copy_mut.optional {
        **val = 999.into();
    }
    assert_eq!(new_bytes, bytes);

    // Note: Cannot use assert_eq! with entire structs due to Vec fields
    // ZeroCopyEq was removed because this struct has Vec fields
    println!("✓ AllDerives Borsh compatibility test passed");
}
