// Edge case: All fields are optional

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct AllOptional {
    pub maybe_a: Option<u32>,
    pub maybe_b: Option<u64>,
    pub maybe_c: Option<Vec<u8>>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = AllOptional {
        maybe_a: Some(42),
        maybe_b: None,
        maybe_c: Some(vec![1, 2, 3]),
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = AllOptional::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = AllOptional::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Test ZeroCopyNew
    let config = AllOptionalConfig {
        maybe_a: true,
        maybe_b: false,
        maybe_c: (true, vec![(); 3]),
    };
    let byte_len = AllOptional::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        AllOptional::new_zero_copy(&mut new_bytes, config).unwrap();
    if let Some(ref mut val) = struct_copy_mut.maybe_a {
        **val = 42u32.into();
    }
    if let Some(ref mut vec_val) = struct_copy_mut.maybe_c {
        *vec_val[0] = 1;
        *vec_val[1] = 2;
        *vec_val[2] = 3;
    }
    assert_eq!(new_bytes, bytes);
}
