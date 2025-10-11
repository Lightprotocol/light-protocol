use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, ZeroCopyEq, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct WithArrays {
    pub buffer: [u8; 32],
    pub data: [u8; 16],
    pub small_array: [u8; 4],
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = WithArrays {
        buffer: [1; 32],
        data: [2; 16],
        small_array: [3, 4, 5, 6],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (struct_copy, remaining) = WithArrays::zero_copy_at(&bytes).unwrap();
    assert_eq!(struct_copy, ref_struct);
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = WithArrays::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = ();
    let byte_len = WithArrays::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        WithArrays::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    for i in 0..32 {
        struct_copy_mut.buffer[i] = 1;
    }
    for i in 0..16 {
        struct_copy_mut.data[i] = 2;
    }
    struct_copy_mut.small_array[0] = 3;
    struct_copy_mut.small_array[1] = 4;
    struct_copy_mut.small_array[2] = 5;
    struct_copy_mut.small_array[3] = 6;
    assert_eq!(new_bytes, bytes);
}
