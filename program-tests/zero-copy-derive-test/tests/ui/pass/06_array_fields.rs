// Edge case: Fixed-size arrays

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct ArrayFields {
    pub small: [u8; 4],
    pub medium: [u32; 16],
    pub large: [u64; 256],
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = ArrayFields {
        small: [1, 2, 3, 4],
        medium: [10; 16],
        large: [100; 256],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = ArrayFields::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = ArrayFields::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = ();
    let byte_len = ArrayFields::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        ArrayFields::new_zero_copy(&mut new_bytes, config).unwrap();
    // set array fields
    struct_copy_mut.small[0] = 1;
    struct_copy_mut.small[1] = 2;
    struct_copy_mut.small[2] = 3;
    struct_copy_mut.small[3] = 4;
    for i in 0..16 {
        struct_copy_mut.medium[i] = 10.into();
    }
    for i in 0..256 {
        struct_copy_mut.large[i] = 100.into();
    }
    assert_eq!(new_bytes, bytes);
}
