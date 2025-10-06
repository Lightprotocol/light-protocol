// Edge case: Multiple bool fields

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, ZeroCopyEq, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct BoolFields {
    pub flag1: bool,
    pub flag2: bool,
    pub flag3: bool,
    pub flag4: bool,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = BoolFields {
        flag1: true,
        flag2: false,
        flag3: true,
        flag4: false,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (struct_copy, _remaining) = BoolFields::zero_copy_at(&bytes).unwrap();
    assert_eq!(struct_copy, ref_struct);
    assert!(_remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, _remaining) = BoolFields::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(_remaining.is_empty());

    // assert byte len
    let config = ();
    let byte_len = BoolFields::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        BoolFields::new_zero_copy(&mut new_bytes, config).unwrap();
    // convert bool to u8 (1 for true, 0 for false)
    struct_copy_mut.flag1 = 1; // true as u8
    struct_copy_mut.flag2 = 0; // false as u8
    struct_copy_mut.flag3 = 1; // true as u8
    struct_copy_mut.flag4 = 0; // false as u8
    assert_eq!(new_bytes, bytes);
}
