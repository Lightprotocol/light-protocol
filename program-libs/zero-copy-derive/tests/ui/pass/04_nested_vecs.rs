// Edge case: Multiple Vec fields
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct NestedVecs {
    pub bytes: Vec<u8>,
    pub nums: Vec<u32>,
    pub more: Vec<u64>,
}

fn main() {}
