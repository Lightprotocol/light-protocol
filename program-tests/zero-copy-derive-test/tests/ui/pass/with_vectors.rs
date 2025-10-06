use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct WithVectors {
    pub data: Vec<u8>,
    pub numbers: Vec<u32>,
    pub flags: Vec<bool>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = WithVectors {
        data: vec![1, 2, 3, 4],
        numbers: vec![10, 20, 30],
        flags: vec![true, false, true],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = WithVectors::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = WithVectors::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = WithVectorsConfig {
        data: 4,
        numbers: 3,
        flags: 3,
    };
    let byte_len = WithVectors::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        WithVectors::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.data[0] = 1;
    struct_copy_mut.data[1] = 2;
    struct_copy_mut.data[2] = 3;
    struct_copy_mut.data[3] = 4;
    struct_copy_mut.numbers[0] = 10.into();
    struct_copy_mut.numbers[1] = 20.into();
    struct_copy_mut.numbers[2] = 30.into();
    struct_copy_mut.flags[0] = 1; // true as u8
    struct_copy_mut.flags[1] = 0; // false as u8
    struct_copy_mut.flags[2] = 1; // true as u8
    assert_eq!(new_bytes, bytes);
}
