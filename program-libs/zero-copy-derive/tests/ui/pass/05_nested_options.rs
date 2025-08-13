// Edge case: Multiple Option fields with optimized types
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct NestedOptions {
    pub opt_u16: Option<u16>,
    pub opt_u32: Option<u32>,
    pub opt_u64: Option<u64>,
}

fn main() {}