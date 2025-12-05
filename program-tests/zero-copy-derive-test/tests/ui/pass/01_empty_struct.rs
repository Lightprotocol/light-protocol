// Edge case: Empty struct (unit struct)

use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct EmptyStruct;

fn main() {}
