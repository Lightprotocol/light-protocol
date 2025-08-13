// Edge case: Triple nested Option
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct TripleNestedOption {
    pub deeply_nested: Option<Option<Option<u32>>>,
    pub also_nested: Option<Option<Vec<u8>>>,
}

fn main() {}
