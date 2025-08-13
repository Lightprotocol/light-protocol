// Edge case: Alternating primitive and dynamic types
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct AlternatingTypes {
    pub p1: u32,
    pub v1: Vec<u8>,
    pub p2: u64,
    pub v2: Vec<u32>,
    pub p3: bool,
    pub o1: Option<u64>,
}

fn main() {}
