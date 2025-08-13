// Edge case: Maximum consecutive meta fields before Vec
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct MaxMetaFields {
    pub m1: u8,
    pub m2: u16,
    pub m3: u32,
    pub m4: u64,
    pub m5: i8,
    pub m6: i16,
    pub m7: i32,
    pub m8: i64,
    pub m9: bool,
    pub m10: [u8; 32],
    pub m11: u32,
    pub m12: u64,
    pub data: Vec<u8>, // Meta boundary
    pub after: u32,
}

fn main() {}
