// Edge case: Multiple Option fields with optimized types
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct NestedOptions {
    pub opt_u16: Option<u16>,
    pub opt_u32: Option<u32>,
    pub opt_u64: Option<u64>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = NestedOptions {
        opt_u16: Some(100),
        opt_u32: Some(200),
        opt_u64: None,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = NestedOptions::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Option fields
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = NestedOptions::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = NestedOptionsConfig {
        opt_u16: true,
        opt_u32: true,
        opt_u64: false,
    };
    let byte_len = NestedOptions::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        NestedOptions::new_zero_copy(&mut new_bytes, config).unwrap();
    // set Option field values - they're already configured via config
    if let Some(ref mut val) = struct_copy_mut.opt_u16 {
        **val = 100.into();
    }
    if let Some(ref mut val) = struct_copy_mut.opt_u32 {
        **val = 200.into();
    }
    // opt_u64 is None, so no assignment needed
    assert_eq!(new_bytes, bytes);
}
