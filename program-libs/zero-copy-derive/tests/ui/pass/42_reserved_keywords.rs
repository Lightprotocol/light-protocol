// Edge case: Fields with names close to reserved keywords
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct ReservedKeywords {
    pub type_: u32,
    pub ref_: u64,
    pub mut_: bool,
    pub fn_: Vec<u8>,
}

fn main() {}
