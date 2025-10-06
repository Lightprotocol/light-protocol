// Edge case: Fields with names close to reserved keywords

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct ReservedKeywords {
    pub type_: u32,
    pub ref_: u64,
    pub mut_: bool,
    pub fn_: Vec<u8>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = ReservedKeywords {
        type_: 10,
        ref_: 20,
        mut_: true,
        fn_: vec![1, 2, 3],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = ReservedKeywords::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = ReservedKeywords::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = ReservedKeywordsConfig { fn_: 3 };
    let byte_len = ReservedKeywords::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        ReservedKeywords::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.type_ = 10.into();
    struct_copy_mut.ref_ = 20.into();
    struct_copy_mut.mut_ = 1; // true as u8
    struct_copy_mut.fn_[0] = 1;
    struct_copy_mut.fn_[1] = 2;
    struct_copy_mut.fn_[2] = 3;
    assert_eq!(new_bytes, bytes);
}
