// Edge case: Single u8 field (smallest primitive)

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, ZeroCopyEq, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct SingleU8 {
    pub value: u8,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = SingleU8 { value: 42 };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (struct_copy, remaining) = SingleU8::zero_copy_at(&bytes).unwrap();
    assert_eq!(struct_copy, ref_struct);
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = SingleU8::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
    // assert byte len
    let config = ();
    let byte_len = SingleU8::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        SingleU8::new_zero_copy(&mut new_bytes, config).unwrap();
    // convert primitive to zero copy type
    struct_copy_mut.value = 42.into();
    assert_eq!(new_bytes, bytes);
}
