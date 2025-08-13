// Edge case: Struct starting with Option (affects meta boundary)
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct OptionFirstField {
    pub optional: Option<u32>, // Meta stops here
    pub value: u64,
    pub more: u32,
    pub data: Vec<u8>,
}

fn main() {}
