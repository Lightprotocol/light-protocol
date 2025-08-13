// Edge case: Enum with single variant
use light_zero_copy_derive::ZeroCopy;

#[derive(ZeroCopy)]
#[repr(C)]
pub enum SingleVariant {
    Only,
}

fn main() {}
