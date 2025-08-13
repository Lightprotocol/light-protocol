// Edge case: Meta boundary at Option field
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct MetaBoundaryOption {
    pub a: u32,
    pub b: u64,
    pub opt: Option<u32>, // Meta boundary here
    pub c: u32,
}

fn main() {}
