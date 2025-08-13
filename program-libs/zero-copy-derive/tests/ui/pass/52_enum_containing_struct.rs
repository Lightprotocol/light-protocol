// Edge case: Enum containing struct type
use light_zero_copy_derive::ZeroCopy;

#[derive(ZeroCopy)]
#[repr(C)]
pub struct InnerData {
    pub field1: u32,
    pub field2: u64,
}

#[derive(ZeroCopy)]
#[repr(C)]
pub enum EnumWithStruct {
    Empty,
    WithStruct(InnerData),
    Another,
}

fn main() {}
