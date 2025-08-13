// Edge case: Primitive fields after Vec (no meta optimization)
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct PrimitiveAfterVec {
    pub data: Vec<u8>,
    pub count: u32,
    pub flag: bool,
}

fn main() {}
