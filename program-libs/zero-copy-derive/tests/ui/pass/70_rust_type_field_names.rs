// Edge case: Field names that are Rust type names
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct RustTypeFieldNames {
    pub u32: u64,            // Field named u32, but type is u64
    pub bool: u32,           // Field named bool, but type is u32
    pub vec: Vec<u8>,        // Field named vec
    pub option: Option<u32>, // Field named option
    pub array: [u8; 10],     // Field named array
}

fn main() {}
