// Edge case: Meta boundary with primitive types
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct MetaBoundaryPrimitive {
    pub a: u32,
    pub b: u64,
    pub c: bool,
    pub vec: Vec<u8>,  // Meta boundary here
    pub d: u32,
    pub e: u64,
}

fn main() {}