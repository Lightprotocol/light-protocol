// Edge case: Empty tuple struct
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct EmptyTuple();

fn main() {}
