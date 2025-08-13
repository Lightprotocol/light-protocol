// Edge case: Mixed field visibility
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct MixedVisibility {
    pub public_field: u32,
    pub(crate) crate_field: u64,
    private_field: Vec<u8>,
}

fn main() {}
