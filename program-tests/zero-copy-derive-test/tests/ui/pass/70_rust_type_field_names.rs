// Edge case: Field names that are Rust type names

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct RustTypeFieldNames {
    pub u32: u64,            // Field named u32, but type is u64
    pub bool: u32,           // Field named bool, but type is u32
    pub vec: Vec<u8>,        // Field named vec
    pub option: Option<u32>, // Field named option
    pub array: [u8; 10],     // Field named array
}

fn main() {
    let original = RustTypeFieldNames {
        u32: 123456789,
        bool: 42,
        vec: vec![1, 2, 3, 4, 5],
        option: Some(999),
        array: [255u8; 10],
    };

    // Test Borsh serialization
    let serialized = original.try_to_vec().unwrap();

    // Test zero_copy_at (read-only)
    let _zero_copy_read = RustTypeFieldNames::zero_copy_at(&serialized).unwrap();

    // Test zero_copy_at_mut (mutable)
    let mut serialized_mut = serialized.clone();
    let _zero_copy_mut = RustTypeFieldNames::zero_copy_at_mut(&mut serialized_mut).unwrap();

    // assert byte len
    let config = RustTypeFieldNamesConfig {
        vec: 5,
        option: true,
    };
    let byte_len = RustTypeFieldNames::byte_len(&config).unwrap();
    assert_eq!(serialized.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        RustTypeFieldNames::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.u32 = 123456789.into();
    struct_copy_mut.bool = 42.into();
    struct_copy_mut.vec[0] = 1;
    struct_copy_mut.vec[1] = 2;
    struct_copy_mut.vec[2] = 3;
    struct_copy_mut.vec[3] = 4;
    struct_copy_mut.vec[4] = 5;
    if let Some(ref mut val) = struct_copy_mut.option {
        **val = 999.into();
    }
    for i in 0..10 {
        struct_copy_mut.array[i] = 255;
    }
    assert_eq!(new_bytes, serialized);

    // Note: Cannot use assert_eq! due to Vec and array fields not implementing ZeroCopyEq
    println!("Borsh compatibility test passed for RustTypeFieldNames");
}
