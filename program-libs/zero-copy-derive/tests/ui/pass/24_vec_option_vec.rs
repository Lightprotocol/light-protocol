// Edge case: Vec, Option, Vec pattern
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct VecOptionVec {
    pub first: Vec<u8>,
    pub middle: Option<u64>,
    pub last: Vec<u32>,
}

fn main() {}