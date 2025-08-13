// Edge case: Field named "bytes" (potential naming conflict)
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct FieldNamedBytes {
    pub bytes: u32,
    pub data: Vec<u8>,
}

fn main() {}