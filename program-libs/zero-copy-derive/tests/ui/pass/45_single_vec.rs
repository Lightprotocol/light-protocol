// Edge case: Single Vec field
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct SingleVec {
    pub data: Vec<u8>,
}

fn main() {}
