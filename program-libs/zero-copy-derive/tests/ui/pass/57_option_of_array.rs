// Edge case: Option containing arrays
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct OptionOfArray {
    pub maybe_bytes: Option<[u8; 32]>,
    pub maybe_nums: Option<[u32; 8]>,
    pub maybe_large: Option<[u64; 64]>,
}

fn main() {}
