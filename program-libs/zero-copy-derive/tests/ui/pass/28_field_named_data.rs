// Edge case: Field named "data" (potential naming conflict)
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct FieldNamedData {
    pub data: u32,
    pub bytes: Vec<u8>,
}

fn main() {}