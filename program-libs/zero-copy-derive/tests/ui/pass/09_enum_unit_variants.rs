// Edge case: Enum with only unit variants
use light_zero_copy_derive::ZeroCopy;

#[derive(ZeroCopy)]
#[repr(C)]
pub enum UnitEnum {
    First,
    Second,
    Third,
    Fourth,
    Fifth,
}

fn main() {}