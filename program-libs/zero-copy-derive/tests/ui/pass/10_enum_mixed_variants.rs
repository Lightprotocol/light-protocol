// Edge case: Enum with mixed variant types
use light_zero_copy_derive::ZeroCopy;

#[derive(ZeroCopy)]
#[repr(C)]
pub enum MixedEnum {
    Empty,
    WithData(u32),
    Another,
}

fn main() {}