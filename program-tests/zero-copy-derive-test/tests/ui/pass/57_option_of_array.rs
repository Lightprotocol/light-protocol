// Edge case: Option containing arrays

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct OptionOfArray {
    pub maybe_bytes: Option<[u8; 32]>,
    pub maybe_nums: Option<[u32; 8]>,
    pub maybe_large: Option<[u64; 64]>,
}

fn main() {
    let instance = OptionOfArray {
        maybe_bytes: Some([42; 32]),
        maybe_nums: Some([1, 2, 3, 4, 5, 6, 7, 8]),
        maybe_large: None,
    };

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();
    let deserialized = OptionOfArray::try_from_slice(&bytes).unwrap();

    // Test zero_copy_at
    let (zero_copy_instance, remaining) = OptionOfArray::zero_copy_at(&bytes).unwrap();
    assert_eq!(
        zero_copy_instance.maybe_bytes.is_some(),
        deserialized.maybe_bytes.is_some()
    );
    assert_eq!(
        zero_copy_instance.maybe_nums.is_some(),
        deserialized.maybe_nums.is_some()
    );
    assert_eq!(
        zero_copy_instance.maybe_large.is_none(),
        deserialized.maybe_large.is_none()
    );
    assert!(remaining.is_empty());

    // Test zero_copy_at_mut
    let mut bytes_mut = bytes.clone();
    let (mut zero_copy_mut, remaining) = OptionOfArray::zero_copy_at_mut(&mut bytes_mut).unwrap();
    if let Some(ref mut bytes) = &mut zero_copy_mut.maybe_bytes {
        bytes[0] = 99;
        assert_eq!(bytes[0], 99);
    }
    assert!(remaining.is_empty());

    // assert byte len
    let config = OptionOfArrayConfig {
        maybe_bytes: (true, ()),
        maybe_nums: (true, ()),
        maybe_large: (false, ()),
    };
    let byte_len = OptionOfArray::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        OptionOfArray::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    if let Some(ref mut arr) = struct_copy_mut.maybe_bytes {
        for i in 0..32 {
            arr[i] = 42;
        }
    }
    if let Some(ref mut arr) = struct_copy_mut.maybe_nums {
        arr[0] = 1.into();
        arr[1] = 2.into();
        arr[2] = 3.into();
        arr[3] = 4.into();
        arr[4] = 5.into();
        arr[5] = 6.into();
        arr[6] = 7.into();
        arr[7] = 8.into();
    }
    assert_eq!(new_bytes, bytes);

    // Note: Cannot use assert_eq! with entire structs due to array fields
    println!("✓ OptionOfArray Borsh compatibility test passed");
}
