// Edge case: Vec containing arrays
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct VecOfArrays {
    pub data: Vec<[u8; 32]>,
}

fn main() {}
