// Edge case: Deep nesting of Options
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct DeepNesting {
    pub nested: Option<Option<u32>>,
}

fn main() {}
