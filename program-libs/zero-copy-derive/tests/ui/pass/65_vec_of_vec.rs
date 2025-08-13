// Edge case: Vec of Vec
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct VecOfVec {
    pub matrix: Vec<Vec<u8>>,
    pub rows: Vec<Vec<u32>>,
}

fn main() {}
