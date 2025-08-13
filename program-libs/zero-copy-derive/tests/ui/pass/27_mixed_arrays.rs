// Edge case: Arrays of different sizes and types
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct MixedArrays {
    pub tiny: [u8; 1],
    pub small: [u16; 8],
    pub medium: [u32; 32],
    pub large: [u64; 128],
}

fn main() {}
