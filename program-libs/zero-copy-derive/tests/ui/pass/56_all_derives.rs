// Edge case: All three derives together
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut, ZeroCopyEq};

#[derive(ZeroCopy, ZeroCopyMut, ZeroCopyEq)]
#[repr(C)]
pub struct AllDerives {
    pub value: u32,
    pub data: Vec<u8>,
    pub optional: Option<u64>,
}

fn main() {}