use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct BasicMutable {
    pub field1: u32,
    pub field2: bool,
    pub data: Vec<u8>,
}

fn main() {}