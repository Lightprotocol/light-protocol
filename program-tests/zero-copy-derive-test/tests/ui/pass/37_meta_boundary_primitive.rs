// Edge case: Meta boundary with primitive types

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct MetaBoundaryPrimitive {
    pub a: u32,
    pub b: u64,
    pub c: bool,
    pub vec: Vec<u8>, // Meta boundary here
    pub d: u32,
    pub e: u64,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = MetaBoundaryPrimitive {
        a: 10,
        b: 20,
        c: true,
        vec: vec![1, 2, 3],
        d: 30,
        e: 40,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = MetaBoundaryPrimitive::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) =
        MetaBoundaryPrimitive::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = MetaBoundaryPrimitiveConfig { vec: 3 };
    let byte_len = MetaBoundaryPrimitive::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        MetaBoundaryPrimitive::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.a = 10.into();
    struct_copy_mut.b = 20.into();
    struct_copy_mut.c = 1; // true as u8
    struct_copy_mut.vec[0] = 1;
    struct_copy_mut.vec[1] = 2;
    struct_copy_mut.vec[2] = 3;
    *struct_copy_mut.d = 30.into();
    *struct_copy_mut.e = 40.into();
    assert_eq!(new_bytes, bytes);
}
