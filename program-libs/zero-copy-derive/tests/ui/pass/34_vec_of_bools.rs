// Edge case: Vec of bools
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct VecOfBools {
    pub flags: Vec<bool>,
}

fn main() {}