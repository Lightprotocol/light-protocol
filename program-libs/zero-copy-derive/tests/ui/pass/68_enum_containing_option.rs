// Edge case: Enum containing Option
use light_zero_copy_derive::ZeroCopy;

#[derive(ZeroCopy)]
#[repr(C)]
pub enum EnumWithOption {
    Empty,
    MaybeData(Option<u32>),
    MaybeVec(Option<Vec<u8>>),
}

fn main() {}
