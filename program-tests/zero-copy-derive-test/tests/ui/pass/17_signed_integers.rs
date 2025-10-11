// Edge case: All signed integer types

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, ZeroCopyEq, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct SignedIntegers {
    pub tiny: i8,
    pub small: i16,
    pub medium: i32,
    pub large: i64,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = SignedIntegers {
        tiny: -1,
        small: -100,
        medium: -1000,
        large: -10000,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (struct_copy, _remaining) = SignedIntegers::zero_copy_at(&bytes).unwrap();
    assert_eq!(struct_copy, ref_struct);
    assert!(_remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, _remaining) = SignedIntegers::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(_remaining.is_empty());

    // assert byte len
    let config = ();
    let byte_len = SignedIntegers::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        SignedIntegers::new_zero_copy(&mut new_bytes, config).unwrap();
    // convert signed integers with .into()
    struct_copy_mut.tiny = (-1).into();
    struct_copy_mut.small = (-100).into();
    struct_copy_mut.medium = (-1000).into();
    struct_copy_mut.large = (-10000).into();
    assert_eq!(new_bytes, bytes);
}
