// Edge case: Fields with numeric suffixes
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct NumericSuffixFields {
    pub field1: u32,
    pub field2: u32,
    pub field3: u32,
    pub data1: Vec<u8>,
    pub data2: Vec<u16>,
}

fn main() {}
