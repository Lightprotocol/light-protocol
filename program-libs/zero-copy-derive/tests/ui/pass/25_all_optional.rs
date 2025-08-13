// Edge case: All fields are optional
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct AllOptional {
    pub maybe_a: Option<u32>,
    pub maybe_b: Option<u64>,
    pub maybe_c: Option<Vec<u8>>,
}

fn main() {}