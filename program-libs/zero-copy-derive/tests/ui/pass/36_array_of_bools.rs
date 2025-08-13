// Edge case: Array of bools
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct ArrayOfBools {
    pub flags: [bool; 32],
}

fn main() {}
