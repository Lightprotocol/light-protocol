// Edge case: Enum containing Vec
use light_zero_copy_derive::ZeroCopy;

#[derive(ZeroCopy)]
#[repr(C)]
pub enum EnumWithVec {
    Empty,
    Data(Vec<u8>),
    MoreData(Vec<u32>),
}

fn main() {}