// Edge case: Deeply nested structs (3+ levels)
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct Level3 {
    pub value: u32,
}

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct Level2 {
    pub inner: Level3,
    pub data: u64,
}

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct Level1 {
    pub inner: Level2,
    pub extra: Vec<u8>,
}

fn main() {}
