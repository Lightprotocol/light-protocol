// Edge case: All fields are Vecs
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct AllVecs {
    pub v1: Vec<u8>,
    pub v2: Vec<u16>,
    pub v3: Vec<u32>,
    pub v4: Vec<u64>,
    pub v5: Vec<bool>,
}

fn main() {}
