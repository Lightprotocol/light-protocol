// Edge case: Fixed-size arrays
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct ArrayFields {
    pub small: [u8; 4],
    pub medium: [u32; 16],
    pub large: [u64; 256],
}

fn main() {}