// Edge case: Zero-sized array
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct ZeroSizedArray {
    pub empty: [u8; 0],
    pub value: u32,
}

fn main() {}
