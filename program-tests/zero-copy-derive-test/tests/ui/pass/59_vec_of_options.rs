// Edge case: Vec containing Options

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct VecOfOptions {
    pub maybe_values: Vec<Option<u32>>,
    pub maybe_bytes: Vec<Option<u8>>,
    pub maybe_large: Vec<Option<u64>>,
}

fn main() {
    let instance = VecOfOptions {
        maybe_values: vec![Some(1), None, Some(3)],
        maybe_bytes: vec![Some(42), Some(99), None],
        maybe_large: vec![None, Some(12345), Some(67890)],
    };

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();

    // Test zero_copy_at
    let (_zero_copy_instance, remaining) = VecOfOptions::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());

    // Test zero_copy_at_mut
    let mut bytes_mut = bytes.clone();
    let (_zero_copy_mut, remaining) = VecOfOptions::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = VecOfOptionsConfig {
        maybe_values: vec![(true, ()), (false, ()), (true, ())],
        maybe_bytes: vec![(true, ()), (true, ()), (false, ())],
        maybe_large: vec![(false, ()), (true, ()), (true, ())],
    };
    let byte_len = VecOfOptions::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        VecOfOptions::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    if let Some(ref mut val) = struct_copy_mut.maybe_values[0] {
        **val = 1.into();
    }
    if let Some(ref mut val) = struct_copy_mut.maybe_values[2] {
        **val = 3.into();
    }
    if let Some(ref mut val) = struct_copy_mut.maybe_bytes[0] {
        **val = 42;
    }
    if let Some(ref mut val) = struct_copy_mut.maybe_bytes[1] {
        **val = 99;
    }
    if let Some(ref mut val) = struct_copy_mut.maybe_large[1] {
        **val = 12345.into();
    }
    if let Some(ref mut val) = struct_copy_mut.maybe_large[2] {
        **val = 67890.into();
    }
    assert_eq!(new_bytes, bytes);

    // Note: Cannot use assert_eq! with entire structs due to Vec fields
    println!("✓ VecOfOptions Borsh compatibility test passed");
}
