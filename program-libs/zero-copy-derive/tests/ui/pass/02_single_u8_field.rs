// Edge case: Single u8 field (smallest primitive)
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct SingleU8 {
    pub value: u8,
}

fn main() {}
