// Edge case: Struct with field that could conflict with lifetime
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct LifetimeName {
    pub a: u32,
    pub lifetime: u64,
    pub static_field: Vec<u8>,
}

fn main() {}