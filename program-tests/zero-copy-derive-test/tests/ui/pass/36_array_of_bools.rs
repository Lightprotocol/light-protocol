// Edge case: Array of bools

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for array fields
#[repr(C)]
pub struct ArrayOfBools {
    pub flags: [bool; 32],
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = ArrayOfBools { flags: [true; 32] };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = ArrayOfBools::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with array fields
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = ArrayOfBools::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = ();
    let byte_len = ArrayOfBools::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        ArrayOfBools::new_zero_copy(&mut new_bytes, config).unwrap();
    // set array of bool values (all true as u8)
    for i in 0..32 {
        struct_copy_mut.flags[i] = 1; // true as u8
    }
    assert_eq!(new_bytes, bytes);
}
