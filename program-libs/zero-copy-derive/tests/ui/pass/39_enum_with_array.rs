// Edge case: Enum variant with array
use light_zero_copy_derive::ZeroCopy;

#[derive(ZeroCopy)]
#[repr(C)]
pub enum EnumWithArray {
    Empty,
    WithArray([u8; 32]),
}

fn main() {}
