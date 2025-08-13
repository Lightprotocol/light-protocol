// Edge case: Fields with underscores
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct UnderscoreFields {
    pub _reserved: u32,
    pub __internal: u64,
    pub normal_field: Vec<u8>,
}

fn main() {}
