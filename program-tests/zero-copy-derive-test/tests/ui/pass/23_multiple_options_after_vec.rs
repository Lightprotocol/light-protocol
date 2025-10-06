// Edge case: Multiple Options after Vec

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct OptionsAfterVec {
    pub data: Vec<u8>,
    pub opt1: Option<u32>,
    pub opt2: Option<u64>,
    pub opt3: Option<u16>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = OptionsAfterVec {
        data: vec![1, 2, 3, 4],
        opt1: Some(42),
        opt2: None,
        opt3: Some(100),
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = OptionsAfterVec::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = OptionsAfterVec::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Test ZeroCopyNew
    let config = OptionsAfterVecConfig {
        data: 4,
        opt1: true,
        opt2: false,
        opt3: true,
    };
    let byte_len = OptionsAfterVec::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        OptionsAfterVec::new_zero_copy(&mut new_bytes, config).unwrap();
    struct_copy_mut.data[0] = 1;
    struct_copy_mut.data[1] = 2;
    struct_copy_mut.data[2] = 3;
    struct_copy_mut.data[3] = 4;
    if let Some(ref mut val) = struct_copy_mut.opt1 {
        **val = 42u32.into();
    }
    if let Some(ref mut val) = struct_copy_mut.opt3 {
        **val = 100u16.into();
    }
    assert_eq!(new_bytes, bytes);
}
