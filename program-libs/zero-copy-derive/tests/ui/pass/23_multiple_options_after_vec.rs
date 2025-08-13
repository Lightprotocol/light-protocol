// Edge case: Multiple Options after Vec
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct OptionsAfterVec {
    pub data: Vec<u8>,
    pub opt1: Option<u32>,
    pub opt2: Option<u64>,
    pub opt3: Option<u16>,
}

fn main() {}
