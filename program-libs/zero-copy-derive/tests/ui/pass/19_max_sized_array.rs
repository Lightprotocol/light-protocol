// Edge case: Maximum practical array size
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct MaxArray {
    pub huge: [u8; 65536],
}

fn main() {}
