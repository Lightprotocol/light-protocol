// Edge case: Vec of bools

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct VecOfBools {
    pub flags: Vec<bool>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = VecOfBools {
        flags: vec![true, false, true],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = VecOfBools::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = VecOfBools::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = VecOfBoolsConfig { flags: 3 };
    let byte_len = VecOfBools::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        VecOfBools::new_zero_copy(&mut new_bytes, config).unwrap();
    // set bool field values (0/1 as u8)
    struct_copy_mut.flags[0] = 1; // true
    struct_copy_mut.flags[1] = 0; // false
    struct_copy_mut.flags[2] = 1; // true
    assert_eq!(new_bytes, bytes);
}
