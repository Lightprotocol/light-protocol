// Edge case: Newtype pattern
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct NewType(pub Vec<u8>);

fn main() {}
