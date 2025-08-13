// Edge case: Struct with only arrays (no Vec/Option)
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct ArraysOnly {
    pub bytes: [u8; 32],
    pub words: [u16; 16],
    pub dwords: [u32; 8],
    pub qwords: [u64; 4],
    pub flags: [bool; 64],
}

fn main() {}
