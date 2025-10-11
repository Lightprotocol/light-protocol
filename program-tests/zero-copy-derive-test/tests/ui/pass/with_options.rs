use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct WithOptions {
    pub maybe_value: Option<u64>,
    pub maybe_flag: Option<bool>,
    pub maybe_small: Option<u16>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = WithOptions {
        maybe_value: Some(42),
        maybe_flag: Some(true),
        maybe_small: None,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = WithOptions::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Option fields
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = WithOptions::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = WithOptionsConfig {
        maybe_value: true,
        maybe_flag: (true, ()),
        maybe_small: false,
    };
    let byte_len = WithOptions::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        WithOptions::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    if let Some(ref mut val) = struct_copy_mut.maybe_value {
        **val = 42.into();
    }
    if let Some(ref mut val) = struct_copy_mut.maybe_flag {
        **val = 1; // true as u8
    }
    assert_eq!(new_bytes, bytes);
}
