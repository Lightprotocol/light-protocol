// Edge case: Single letter field names
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct SingleLetterFields {
    pub a: u8,
    pub b: u16,
    pub c: u32,
    pub d: u64,
    pub e: Vec<u8>,
    pub f: Option<u32>,
}

fn main() {}
