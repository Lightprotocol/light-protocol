// Edge case: Tuple struct with single field
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct SingleFieldTuple(pub u32);

fn main() {}
