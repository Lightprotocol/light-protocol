// Edge case: Arrays of different sizes and types

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for array fields
#[repr(C)]
pub struct MixedArrays {
    pub tiny: [u8; 1],
    pub small: [u16; 8],
    pub medium: [u32; 32],
    pub large: [u64; 128],
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = MixedArrays {
        tiny: [1],
        small: [2; 8],
        medium: [3; 32],
        large: [4; 128],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = MixedArrays::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = MixedArrays::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = ();
    let byte_len = MixedArrays::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        MixedArrays::new_zero_copy(&mut new_bytes, config).unwrap();
    // set array values
    struct_copy_mut.tiny[0] = 1;
    for i in 0..8 {
        struct_copy_mut.small[i] = 2.into();
    }
    for i in 0..32 {
        struct_copy_mut.medium[i] = 3.into();
    }
    for i in 0..128 {
        struct_copy_mut.large[i] = 4.into();
    }
    assert_eq!(new_bytes, bytes);
}
