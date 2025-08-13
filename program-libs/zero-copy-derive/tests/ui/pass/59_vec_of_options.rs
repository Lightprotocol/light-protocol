// Edge case: Vec containing Options
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct VecOfOptions {
    pub maybe_values: Vec<Option<u32>>,
    pub maybe_bytes: Vec<Option<u8>>,
    pub maybe_large: Vec<Option<u64>>,
}

fn main() {}
