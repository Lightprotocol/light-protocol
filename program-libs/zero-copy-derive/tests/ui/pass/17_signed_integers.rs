// Edge case: All signed integer types
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct SignedIntegers {
    pub tiny: i8,
    pub small: i16,
    pub medium: i32,
    pub large: i64,
}

fn main() {}
