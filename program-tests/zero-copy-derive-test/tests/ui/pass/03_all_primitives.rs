// Edge case: All primitive types

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, ZeroCopyEq, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct AllPrimitives {
    pub a: u8,
    pub b: u16,
    pub c: u32,
    pub d: u64,
    pub e: i8,
    pub f: i16,
    pub g: i32,
    pub h: i64,
    pub i: bool,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = AllPrimitives {
        a: 1,
        b: 2,
        c: 3,
        d: 4,
        e: -1,
        f: -2,
        g: -3,
        h: -4,
        i: true,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (struct_copy, _remaining) = AllPrimitives::zero_copy_at(&bytes).unwrap();
    assert_eq!(ref_struct, struct_copy);
    assert!(_remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, _remaining) = AllPrimitives::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(_remaining.is_empty());

    // assert byte len
    let config = ();
    let byte_len = AllPrimitives::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        AllPrimitives::new_zero_copy(&mut new_bytes, config).unwrap();
    // convert primitives to zero copy types
    struct_copy_mut.a = 1.into();
    struct_copy_mut.b = 2.into();
    struct_copy_mut.c = 3.into();
    struct_copy_mut.d = 4.into();
    struct_copy_mut.e = (-1).into();
    struct_copy_mut.f = (-2).into();
    struct_copy_mut.g = (-3).into();
    struct_copy_mut.h = (-4).into();
    struct_copy_mut.i = 1; // true as u8
    assert_eq!(new_bytes, bytes);
}
