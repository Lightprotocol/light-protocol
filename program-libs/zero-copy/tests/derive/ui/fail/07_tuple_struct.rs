// Edge case: Tuple struct
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct TupleStruct(pub u32, pub Vec<u8>, pub Option<u64>);

fn main() {}