// Edge case: Meta boundary at Option field

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, ZeroCopyEq, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct MetaBoundaryOption {
    pub a: u32,
    pub b: u64,
    pub opt: Option<u32>, // Meta boundary here
    pub c: u32,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = MetaBoundaryOption {
        a: 10,
        b: 20,
        opt: Some(42),
        c: 30,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (struct_copy, remaining) = MetaBoundaryOption::zero_copy_at(&bytes).unwrap();
    assert_eq!(struct_copy, ref_struct);
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) =
        MetaBoundaryOption::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = MetaBoundaryOptionConfig { opt: true };
    let byte_len = MetaBoundaryOption::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        MetaBoundaryOption::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.a = 10.into();
    struct_copy_mut.b = 20.into();
    if let Some(ref mut val) = struct_copy_mut.opt {
        **val = 42.into();
    }
    *struct_copy_mut.c = 30.into();
    assert_eq!(new_bytes, bytes);
}
