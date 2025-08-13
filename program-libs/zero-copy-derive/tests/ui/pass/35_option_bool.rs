// Edge case: Option<bool>
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct OptionBool {
    pub maybe_flag: Option<bool>,
}

fn main() {}
