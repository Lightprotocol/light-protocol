// Edge case: Struct containing other zero-copy structs
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct Inner {
    pub value: u32,
}

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct Outer {
    pub inner: Inner,
    pub data: Vec<u8>,
}

fn main() {}
