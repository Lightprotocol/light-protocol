// Edge case: Meta boundary at Option field
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
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
}
