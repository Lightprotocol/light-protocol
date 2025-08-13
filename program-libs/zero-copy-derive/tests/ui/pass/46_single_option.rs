// Edge case: Single Option field
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct SingleOption {
    pub maybe: Option<u64>,
}

fn main() {}
