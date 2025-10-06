// Edge case: Maximum practical array size

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, ZeroCopyEq, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct MaxArray {
    pub huge: [u8; 65536],
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = MaxArray { huge: [42; 65536] };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (struct_copy, remaining) = MaxArray::zero_copy_at(&bytes).unwrap();
    assert_eq!(struct_copy, ref_struct);
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = MaxArray::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = ();
    let byte_len = MaxArray::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, remaining) = MaxArray::new_zero_copy(&mut new_bytes, config).unwrap();
    assert!(remaining.is_empty());
    // set all elements of the large array
    for i in 0..65536 {
        struct_copy_mut.huge[i] = 42;
    }
    assert_eq!(new_bytes, bytes);
}
