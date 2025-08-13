// Edge case: Option containing Vec
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct OptionVec {
    pub maybe_data: Option<Vec<u8>>,
}

fn main() {}